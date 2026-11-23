//! GDI+ bridge used to draw the original PNG assets and the embedded Poppins font
//! directly from memory (no temporary files, no external crate).

use crate::settings::{FontKind, Lang};
use crate::win::{self, HDC, HICON};
use core::ffi::c_void;

const PNG_ON: &[u8] = include_bytes!("../assets/bgon.png");
const PNG_OFF: &[u8] = include_bytes!("../assets/bgoff.png");
const PNG_LOGO: &[u8] = include_bytes!("../assets/IndiCapsRustLogo.png");
const FONT_POPPINS: &[u8] = include_bytes!("../assets/Poppins-Regular.ttf");

// PixelFormat32bppARGB = 10 | (32 << 8) | Alpha | GDI | Canonical.
const PIXEL_FORMAT_32_ARGB: i32 = 10 | (32 << 8) | 0x0004_0000 | 0x0002_0000 | 0x0020_0000;
const INTERPOLATION_HIGH_QUALITY_BICUBIC: i32 = 7;
const SMOOTHING_HIGH_QUALITY: i32 = 2;
const PIXEL_OFFSET_HIGH_QUALITY: i32 = 2;
const COMPOSITING_HIGH_QUALITY: i32 = 2;
const UNIT_PIXEL: i32 = 2;
const UNIT_POINT: i32 = 3;
const FILL_MODE_ALTERNATE: i32 = 0;
const TEXT_RENDERING_ANTIALIAS_GRID_FIT: i32 = 3;
const TEXT_RENDERING_ANTIALIAS: i32 = 4;
const STRING_ALIGNMENT_NEAR: i32 = 0;
const STRING_ALIGNMENT_CENTER: i32 = 1;
const STRING_FORMAT_FLAGS_DIRECTION_RTL: i32 = 0x0000_0004;
const STRING_FORMAT_FLAGS_NO_WRAP: i32 = 0x0000_1000;

const FONT_COUNT: usize = FontKind::SystemLatin as usize + 1;

#[repr(C)]
struct GdiplusStartupInput {
    version: u32,
    callback: *mut c_void,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct RectF {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Minimal COM vtable header needed to release an `IStream` created by
/// `CreateStreamOnHGlobal`. The third slot is `Release`.
#[repr(C)]
struct ComStream {
    vtbl: *const *const c_void,
}

type Gp = *mut c_void;

pub struct Gfx {
    token: usize,
    com_initialized: bool,
    on: Gp,
    off: Gp,
    logo: Gp,
    collection: Gp,
    families: [Gp; FONT_COUNT],
}

/// Converts a Win32 COLORREF (0x00BBGGRR) to the ARGB value used by GDI+.
#[inline]
pub fn argb(colorref: u32) -> u32 {
    0xFF00_0000 | ((colorref & 0xFF) << 16) | (colorref & 0x00FF_00) | ((colorref >> 16) & 0xFF)
}

/// Anti-aliased drawing surface backed by a 32bpp GDI+ bitmap.
pub struct Canvas<'a> {
    bitmap: Gp,
    graphics: Gp,
    families: [Gp; FONT_COUNT],
    font_kind: FontKind,
    rtl: bool,
    _gfx: core::marker::PhantomData<&'a Gfx>,
}

impl<'a> Canvas<'a> {
    pub unsafe fn clear(&mut self, color: u32) {
        GdipGraphicsClear(self.graphics, color);
    }

    /// Switches the font/script used by subsequent `text` calls. Language names
    /// in the dropdown use their own script even when the UI language differs.
    pub fn set_language(&mut self, lang: Lang) {
        self.font_kind = lang.font_kind();
        self.rtl = lang.is_rtl();
    }

    pub unsafe fn round_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        fill: u32,
        border: u32,
        border_width: f32,
    ) {
        let mut path: Gp = core::ptr::null_mut();
        if GdipCreatePath(FILL_MODE_ALTERNATE, &mut path) != 0 {
            return;
        }
        let d = r * 2.0;
        GdipAddPathArc(path, x, y, d, d, 180.0, 90.0);
        GdipAddPathArc(path, x + w - d, y, d, d, 270.0, 90.0);
        GdipAddPathArc(path, x + w - d, y + h - d, d, d, 0.0, 90.0);
        GdipAddPathArc(path, x, y + h - d, d, d, 90.0, 90.0);
        GdipClosePathFigure(path);

        let mut brush: Gp = core::ptr::null_mut();
        if GdipCreateSolidFill(fill, &mut brush) == 0 {
            GdipFillPath(self.graphics, brush, path);
            GdipDeleteBrush(brush);
        }
        if border_width > 0.0 {
            let mut pen: Gp = core::ptr::null_mut();
            if GdipCreatePen1(border, border_width, UNIT_PIXEL, &mut pen) == 0 {
                GdipDrawPath(self.graphics, pen, path);
                GdipDeletePen(pen);
            }
        }
        GdipDeletePath(path);
    }

    pub unsafe fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: u32, width: f32) {
        let mut pen: Gp = core::ptr::null_mut();
        if GdipCreatePen1(color, width, UNIT_PIXEL, &mut pen) == 0 {
            GdipDrawLine(self.graphics, pen, x1, y1, x2, y2);
            GdipDeletePen(pen);
        }
    }

    /// Draws one of the images owned by `Gfx` into this canvas.
    pub unsafe fn image(&mut self, image: *mut c_void, x: f32, y: f32, w: f32, h: f32) {
        if image.is_null() {
            return;
        }
        GdipDrawImageRectI(self.graphics, image, x as i32, y as i32, w as i32, h as i32);
    }

    pub unsafe fn ellipse(&mut self, x: f32, y: f32, w: f32, h: f32, fill: u32, border: u32, border_width: f32) {
        let mut brush: Gp = core::ptr::null_mut();
        if GdipCreateSolidFill(fill, &mut brush) == 0 {
            GdipFillEllipse(self.graphics, brush, x, y, w, h);
            GdipDeleteBrush(brush);
        }
        if border_width > 0.0 {
            let mut pen: Gp = core::ptr::null_mut();
            if GdipCreatePen1(border, border_width, UNIT_PIXEL, &mut pen) == 0 {
                GdipDrawEllipse(self.graphics, pen, x, y, w, h);
                GdipDeletePen(pen);
            }
        }
    }

    pub unsafe fn text(
        &mut self,
        label: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        px: f32,
        color: u32,
        centered: bool,
    ) {
        let mut font: Gp = core::ptr::null_mut();
        // Pixel units keep the small panel labels predictable at any DPI and
        // let them render larger than the old 10-11 pt sizes.
        GdipCreateFont(self.families[self.font_kind as usize], px, 0, UNIT_PIXEL, &mut font);
        let mut format: Gp = core::ptr::null_mut();
        let mut flags = STRING_FORMAT_FLAGS_NO_WRAP;
        if self.rtl {
            flags |= STRING_FORMAT_FLAGS_DIRECTION_RTL;
        }
        if GdipCreateStringFormat(flags, 0, &mut format) != 0 {
            if !font.is_null() {
                GdipDeleteFont(font);
            }
            return;
        }
        GdipSetStringFormatAlign(format, if centered { STRING_ALIGNMENT_CENTER } else { STRING_ALIGNMENT_NEAR });
        GdipSetStringFormatLineAlign(format, STRING_ALIGNMENT_CENTER);
        let mut brush: Gp = core::ptr::null_mut();
        if GdipCreateSolidFill(color, &mut brush) == 0 {
            let rect = RectF { x, y, w, h };
            GdipDrawString(self.graphics, win::wide(label).as_ptr(), -1, font, &rect, format, brush);
            GdipDeleteBrush(brush);
        }
        GdipDeleteStringFormat(format);
        if !font.is_null() {
            GdipDeleteFont(font);
        }
    }
}

impl<'a> Drop for Canvas<'a> {
    fn drop(&mut self) {
        unsafe {
            if !self.graphics.is_null() {
                GdipDeleteGraphics(self.graphics);
            }
            if !self.bitmap.is_null() {
                GdipDisposeImage(self.bitmap);
            }
        }
    }
}

impl Gfx {
    pub unsafe fn new() -> Option<Self> {
        // The PNG codec uses COM; GDI+ startup alone does not initialise it.
        // S_FALSE also counts as success and still needs a matching Uninitialize.
        let com_initialized = CoInitializeEx(core::ptr::null_mut(), 0) >= 0; // COINIT_APARTMENTTHREADED
        let input = GdiplusStartupInput {
            version: 1,
            callback: core::ptr::null_mut(),
            suppress_background_thread: 0,
            suppress_external_codecs: 0,
        };
        let mut token = 0usize;
        if GdiplusStartup(&mut token, &input, core::ptr::null_mut()) != 0 {
            if com_initialized {
                CoUninitialize();
            }
            return None;
        }
        let on = load_image(PNG_ON);
        let off = load_image(PNG_OFF);
        let logo = load_image(PNG_LOGO);

        let mut collection: Gp = core::ptr::null_mut();
        GdipNewPrivateFontCollection(&mut collection);
        GdipPrivateAddMemoryFont(
            collection,
            FONT_POPPINS.as_ptr() as *const c_void,
            FONT_POPPINS.len() as i32,
        );

        let mut families = [core::ptr::null_mut(); FONT_COUNT];
        families[FontKind::Poppins as usize] =
            load_family(collection, &["Poppins", "Segoe UI"]);
        families[FontKind::CjkSc as usize] =
            load_family(core::ptr::null_mut(), &["Microsoft YaHei", "SimHei", "Segoe UI"]);
        families[FontKind::CjkJp as usize] =
            load_family(core::ptr::null_mut(), &["Yu Gothic", "Meiryo", "MS Gothic", "Segoe UI"]);
        families[FontKind::CjkKr as usize] =
            load_family(core::ptr::null_mut(), &["Malgun Gothic", "Gulim", "Segoe UI"]);
        families[FontKind::Cyrillic as usize] =
            load_family(core::ptr::null_mut(), &["Segoe UI", "Arial"]);
        families[FontKind::Arabic as usize] =
            load_family(core::ptr::null_mut(), &["Segoe UI", "Arial"]);
        families[FontKind::Indic as usize] =
            load_family(core::ptr::null_mut(), &["Nirmala UI", "Mangal", "Segoe UI"]);
        families[FontKind::Thai as usize] =
            load_family(core::ptr::null_mut(), &["Leelawadee UI", "Segoe UI"]);
        families[FontKind::SystemLatin as usize] =
            load_family(core::ptr::null_mut(), &["Segoe UI", "Arial"]);
        Some(Self { token, com_initialized, on, off, logo, collection, families })
    }

    pub fn image(&self, on: bool) -> Gp {
        if on { self.on } else { self.off }
    }

    /// Official logo image, drawn in the settings panel header.
    pub fn logo(&self) -> *mut c_void {
        self.logo
    }

    /// Creates an anti-aliased GDI+ canvas over an existing 32bpp DIB.
    pub unsafe fn canvas(&self, bits: *mut u8, w: i32, h: i32) -> Option<Canvas<'_>> {
        let mut bitmap: Gp = core::ptr::null_mut();
        if GdipCreateBitmapFromScan0(w, h, w * 4, PIXEL_FORMAT_32_ARGB, bits, &mut bitmap) != 0 {
            return None;
        }
        let mut graphics: Gp = core::ptr::null_mut();
        if GdipGetImageGraphicsContext(bitmap, &mut graphics) != 0 {
            GdipDisposeImage(bitmap);
            return None;
        }
        configure(graphics);
        // Small panel labels look sharper without grid fitting; the larger
        // overlay caption keeps the grid-fitted hinting.
        GdipSetTextRenderingHint(graphics, TEXT_RENDERING_ANTIALIAS);
        Some(Canvas {
            bitmap,
            graphics,
            families: self.families,
            font_kind: FontKind::Poppins,
            rtl: false,
            _gfx: core::marker::PhantomData,
        })
    }

    /// Draws the original PNG and the translated caption on a 32bpp DIB surface.
    pub unsafe fn render_overlay(
        &self,
        bits: *mut u8,
        w: i32,
        h: i32,
        on: bool,
        caption: &str,
        pt: f32,
        lang: Lang,
    ) {
        let mut bitmap: Gp = core::ptr::null_mut();
        if GdipCreateBitmapFromScan0(
            w,
            h,
            w * 4,
            PIXEL_FORMAT_32_ARGB,
            bits,
            &mut bitmap,
        ) != 0
        {
            return;
        }
        let mut gfx: Gp = core::ptr::null_mut();
        if GdipGetImageGraphicsContext(bitmap, &mut gfx) != 0 {
            GdipDisposeImage(bitmap);
            return;
        }
        configure(gfx);
        GdipSetTextRenderingHint(gfx, TEXT_RENDERING_ANTIALIAS_GRID_FIT);
        GdipGraphicsClear(gfx, 0);
        GdipDrawImageRectI(gfx, self.image(on), 0, 0, w, h);

        let rect = RectF { x: 0.0, y: 0.0, w: w as f32, h: h as f32 };
        let font = self.font(pt, lang.font_kind());
        let format = text_format(STRING_ALIGNMENT_CENTER, lang.is_rtl());
        if !font.is_null() && !format.is_null() {
            let mut shadow: Gp = core::ptr::null_mut();
            let mut white: Gp = core::ptr::null_mut();
            GdipCreateSolidFill(0x9600_0000, &mut shadow);
            GdipCreateSolidFill(0xFFFF_FFFF, &mut white);
            let text = win::wide(caption);
            let mut shifted = rect;
            shifted.y += 2.0;
            GdipDrawString(gfx, text.as_ptr(), -1, font, &shifted, format, shadow);
            GdipDrawString(gfx, text.as_ptr(), -1, font, &rect, format, white);
            if !shadow.is_null() {
                GdipDeleteBrush(shadow);
            }
            if !white.is_null() {
                GdipDeleteBrush(white);
            }
        }
        if !format.is_null() {
            GdipDeleteStringFormat(format);
        }
        if !font.is_null() {
            GdipDeleteFont(font);
        }
        GdipDeleteGraphics(gfx);
        GdipDisposeImage(bitmap);
    }

    /// Draws the original indicator artwork onto an existing GDI+ canvas.
    /// Used by the position editor to preview the real notification.
    pub unsafe fn draw_indicator(
        &self,
        canvas: &mut Canvas<'_>,
        on: bool,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        GdipDrawImageRectI(
            canvas.graphics,
            self.image(on),
            x.round() as i32,
            y.round() as i32,
            w.round() as i32,
            h.round() as i32,
        );
    }

    /// Builds a 32x32 tray icon from the original logo PNG.
    pub unsafe fn tray_icon(&self) -> HICON {
        let mut bitmap: Gp = core::ptr::null_mut();
        let mut gfx: Gp = core::ptr::null_mut();
        let mut icon: HICON = core::ptr::null_mut();
        if GdipCreateBitmapFromScan0(
            32,
            32,
            32 * 4,
            PIXEL_FORMAT_32_ARGB,
            core::ptr::null_mut(),
            &mut bitmap,
        ) != 0
        {
            return icon;
        }
        if GdipGetImageGraphicsContext(bitmap, &mut gfx) != 0 {
            GdipDisposeImage(bitmap);
            return icon;
        }
        configure(gfx);
        GdipGraphicsClear(gfx, 0);
        GdipDrawImageRectI(gfx, self.logo, 0, 0, 32, 32);
        GdipCreateHICONFromBitmap(bitmap, &mut icon);
        GdipDeleteGraphics(gfx);
        GdipDisposeImage(bitmap);
        icon
    }

    fn font(&self, pt: f32, kind: FontKind) -> Gp {
        let mut font: Gp = core::ptr::null_mut();
        unsafe { GdipCreateFont(self.families[kind as usize], pt, 0, UNIT_POINT, &mut font) };
        font
    }

    /// Draws one line of text on an existing HDC (used by the settings panel).
    /// `color` is a Win32 COLORREF (0x00BBGGRR).
    pub unsafe fn draw_text(
        &self,
        hdc: HDC,
        text: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        pt: f32,
        color: u32,
        centered: bool,
        lang: Lang,
    ) {
        let mut gfx: Gp = core::ptr::null_mut();
        if GdipCreateFromHDC(hdc, &mut gfx) != 0 {
            return;
        }
        configure(gfx);
        GdipSetTextRenderingHint(gfx, TEXT_RENDERING_ANTIALIAS_GRID_FIT);
        let font = self.font(pt, lang.font_kind());
        let format = text_format(
            if centered { STRING_ALIGNMENT_CENTER } else { STRING_ALIGNMENT_NEAR },
            lang.is_rtl(),
        );
        if !font.is_null() && !format.is_null() {
            let mut brush: Gp = core::ptr::null_mut();
            let argb = 0xFF00_0000
                | ((color & 0xFF) << 16)
                | (color & 0x00FF_00)
                | ((color >> 16) & 0xFF);
            GdipCreateSolidFill(argb, &mut brush);
            let rect = RectF { x, y, w, h };
            GdipDrawString(gfx, win::wide(text).as_ptr(), -1, font, &rect, format, brush);
            if !brush.is_null() {
                GdipDeleteBrush(brush);
            }
        }
        if !format.is_null() {
            GdipDeleteStringFormat(format);
        }
        if !font.is_null() {
            GdipDeleteFont(font);
        }
        GdipDeleteGraphics(gfx);
    }
}

impl Drop for Gfx {
    fn drop(&mut self) {
        unsafe {
            for image in [self.on, self.off, self.logo] {
                if !image.is_null() {
                    GdipDisposeImage(image);
                }
            }
            for family in self.families {
                if !family.is_null() {
                    GdipDeleteFontFamily(family);
                }
            }
            if !self.collection.is_null() {
                GdipDeletePrivateFontCollection(&mut self.collection);
            }
            GdiplusShutdown(self.token);
            if self.com_initialized {
                CoUninitialize();
            }
        }
    }
}

unsafe fn configure(gfx: Gp) {
    GdipSetInterpolationMode(gfx, INTERPOLATION_HIGH_QUALITY_BICUBIC);
    GdipSetSmoothingMode(gfx, SMOOTHING_HIGH_QUALITY);
    GdipSetPixelOffsetMode(gfx, PIXEL_OFFSET_HIGH_QUALITY);
    GdipSetCompositingQuality(gfx, COMPOSITING_HIGH_QUALITY);
}

unsafe fn text_format(align: i32, rtl: bool) -> Gp {
    let mut format: Gp = core::ptr::null_mut();
    let mut flags = STRING_FORMAT_FLAGS_NO_WRAP;
    if rtl {
        flags |= STRING_FORMAT_FLAGS_DIRECTION_RTL;
    }
    if GdipCreateStringFormat(flags, 0, &mut format) != 0 {
        return core::ptr::null_mut();
    }
    GdipSetStringFormatAlign(format, align);
    GdipSetStringFormatLineAlign(format, STRING_ALIGNMENT_CENTER);
    format
}

unsafe fn load_family(collection: Gp, names: &[&str]) -> Gp {
    for name in names {
        let wide_name = win::wide(name);
        let mut family: Gp = core::ptr::null_mut();
        if GdipCreateFontFamilyFromName(wide_name.as_ptr(), collection, &mut family) == 0 {
            return family;
        }
    }
    core::ptr::null_mut()
}

unsafe fn release_stream(stream: *mut c_void) {
    if stream.is_null() {
        return;
    }
    let vtbl = (*(stream as *const ComStream)).vtbl;
    if vtbl.is_null() {
        return;
    }
    let release: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*vtbl.add(2));
    release(stream);
}

unsafe fn load_image(bytes: &[u8]) -> Gp {
    const GMEM_MOVEABLE: u32 = 0x0002;
    let mut image: Gp = core::ptr::null_mut();
    let hglobal = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
    if hglobal.is_null() {
        return image;
    }
    let dst = GlobalLock(hglobal) as *mut u8;
    if dst.is_null() {
        GlobalFree(hglobal);
        return image;
    }
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
    GlobalUnlock(hglobal);
    let mut stream: *mut c_void = core::ptr::null_mut();
    if CreateStreamOnHGlobal(hglobal, 1, &mut stream) != 0 {
        // The stream did not take ownership, so we still have to free it.
        GlobalFree(hglobal);
        return image;
    }
    GdipLoadImageFromStream(stream, &mut image);
    release_stream(stream); // also frees the HGLOBAL
    image
}

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> i32;
    fn CoUninitialize();
    fn CreateStreamOnHGlobal(
        hglobal: *mut c_void,
        delete_on_release: i32,
        stream: *mut *mut c_void,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
    fn GlobalLock(hglobal: *mut c_void) -> *mut c_void;
    fn GlobalUnlock(hglobal: *mut c_void) -> i32;
    fn GlobalFree(hglobal: *mut c_void) -> *mut c_void;
}

#[link(name = "gdiplus")]
extern "system" {
    fn GdiplusStartup(token: *mut usize, input: *const GdiplusStartupInput, output: *mut c_void) -> i32;
    fn GdiplusShutdown(token: usize);
    fn GdipLoadImageFromStream(stream: *mut c_void, image: *mut Gp) -> i32;
    fn GdipDisposeImage(image: Gp) -> i32;
    fn GdipCreateBitmapFromScan0(
        width: i32,
        height: i32,
        stride: i32,
        format: i32,
        scan0: *mut u8,
        bitmap: *mut Gp,
    ) -> i32;
    fn GdipGetImageGraphicsContext(image: Gp, graphics: *mut Gp) -> i32;
    fn GdipCreateFromHDC(hdc: HDC, graphics: *mut Gp) -> i32;
    fn GdipDeleteGraphics(graphics: Gp) -> i32;
    fn GdipGraphicsClear(graphics: Gp, color: u32) -> i32;
    fn GdipSetInterpolationMode(graphics: Gp, mode: i32) -> i32;
    fn GdipSetSmoothingMode(graphics: Gp, mode: i32) -> i32;
    fn GdipSetPixelOffsetMode(graphics: Gp, mode: i32) -> i32;
    fn GdipSetCompositingQuality(graphics: Gp, quality: i32) -> i32;
    fn GdipSetTextRenderingHint(graphics: Gp, hint: i32) -> i32;
    fn GdipCreatePath(fill_mode: i32, path: *mut Gp) -> i32;
    fn GdipAddPathArc(path: Gp, x: f32, y: f32, w: f32, h: f32, start: f32, sweep: f32) -> i32;
    fn GdipClosePathFigure(path: Gp) -> i32;
    fn GdipDeletePath(path: Gp) -> i32;
    fn GdipFillPath(graphics: Gp, brush: Gp, path: Gp) -> i32;
    fn GdipDrawPath(graphics: Gp, pen: Gp, path: Gp) -> i32;
    fn GdipCreatePen1(color: u32, width: f32, unit: i32, pen: *mut Gp) -> i32;
    fn GdipDeletePen(pen: Gp) -> i32;
    fn GdipDrawLine(graphics: Gp, pen: Gp, x1: f32, y1: f32, x2: f32, y2: f32) -> i32;
    fn GdipFillEllipse(graphics: Gp, brush: Gp, x: f32, y: f32, w: f32, h: f32) -> i32;
    fn GdipDrawEllipse(graphics: Gp, pen: Gp, x: f32, y: f32, w: f32, h: f32) -> i32;
    fn GdipDrawImageRectI(graphics: Gp, image: Gp, x: i32, y: i32, w: i32, h: i32) -> i32;
    fn GdipCreateHICONFromBitmap(bitmap: Gp, icon: *mut HICON) -> i32;
    fn GdipNewPrivateFontCollection(collection: *mut Gp) -> i32;
    fn GdipDeletePrivateFontCollection(collection: *mut Gp) -> i32;
    fn GdipPrivateAddMemoryFont(collection: Gp, memory: *const c_void, length: i32) -> i32;
    fn GdipCreateFontFamilyFromName(name: *const u16, collection: Gp, family: *mut Gp) -> i32;
    fn GdipDeleteFontFamily(family: Gp) -> i32;
    fn GdipCreateFont(family: Gp, size: f32, style: i32, unit: i32, font: *mut Gp) -> i32;
    fn GdipDeleteFont(font: Gp) -> i32;
    fn GdipCreateStringFormat(flags: i32, language: u16, format: *mut Gp) -> i32;
    fn GdipSetStringFormatAlign(format: Gp, align: i32) -> i32;
    fn GdipSetStringFormatLineAlign(format: Gp, align: i32) -> i32;
    fn GdipDeleteStringFormat(format: Gp) -> i32;
    fn GdipCreateSolidFill(color: u32, brush: *mut Gp) -> i32;
    fn GdipDeleteBrush(brush: Gp) -> i32;
    fn GdipDrawString(
        graphics: Gp,
        text: *const u16,
        length: i32,
        font: Gp,
        rect: *const RectF,
        format: Gp,
        brush: Gp,
    ) -> i32;
}
