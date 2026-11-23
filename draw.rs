use crate::win;
use core::ffi::c_void;

/// A 32-bit top-down DIB. Rows are stored top-first, BGRA, straight alpha until
/// `premultiply()` is called (required by UpdateLayeredWindow with AC_SRC_ALPHA).
pub struct Surface {
    pub bitmap: win::HBITMAP,
    pub bits: *mut u8,
    pub w: i32,
    pub h: i32,
}

impl Surface {
    pub fn new(w: i32, h: i32) -> Self {
        // A failed allocation must never leave a null `bits` pointer behind:
        // every painter writes through it. Report failure as a zero-sized,
        // empty surface so callers can skip rendering safely.
        if w <= 0 || h <= 0 {
            return Self { bitmap: core::ptr::null_mut(), bits: core::ptr::null_mut(), w: 0, h: 0 };
        }
        unsafe {
            let info = win::BITMAPINFO::new(w, -h);
            let mut bits: *mut c_void = core::ptr::null_mut();
            let bitmap = win::CreateDIBSection(
                core::ptr::null_mut(),
                &info,
                0,
                &mut bits,
                core::ptr::null_mut(),
                0,
            );
            if bitmap.is_null() || bits.is_null() {
                if !bitmap.is_null() {
                    win::DeleteObject(bitmap);
                }
                return Self { bitmap: core::ptr::null_mut(), bits: core::ptr::null_mut(), w: 0, h: 0 };
            }
            let s = Self { bitmap, bits: bits as *mut u8, w, h };
            s.clear();
            s
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.bitmap.is_null() && !self.bits.is_null()
    }

    pub fn clear(&self) {
        if !self.is_valid() {
            return;
        }
        unsafe { core::ptr::write_bytes(self.bits, 0, (self.w * self.h * 4) as usize) }
    }

    #[inline]
    pub fn put(&self, x: i32, y: i32, px: [u8; 4]) {
        if !self.is_valid() || x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        unsafe {
            let p = self.bits.add(((y * self.w + x) * 4) as usize);
            *p = px[0];
            *p.add(1) = px[1];
            *p.add(2) = px[2];
            *p.add(3) = px[3];
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> [u8; 4] {
        if !self.is_valid() || x < 0 || y < 0 || x >= self.w || y >= self.h {
            return [0; 4];
        }
        unsafe {
            let p = self.bits.add(((y * self.w + x) * 4) as usize);
            [*p, *p.add(1), *p.add(2), *p.add(3)]
        }
    }

    /// Returns white text coverage in 0..=255, using GDI antialiasing.
    pub fn text_mask(&self, text: &str, pt: f32, dpi: u32, bold: bool) -> Vec<u8> {
        if !self.is_valid() {
            return Vec::new();
        }
        unsafe {
            let mask = Surface::new(self.w, self.h);
            if !mask.is_valid() {
                return Vec::new();
            }
            let screen = win::GetDC(core::ptr::null_mut());
            let dc = win::CreateCompatibleDC(screen);
            let old = win::SelectObject(dc, mask.bitmap);
            let font = make_font(pt, dpi, bold);
            let old_font = win::SelectObject(dc, font);

            win::SetBkMode(dc, win::OPAQUE);
            win::SetBkColor(dc, win::rgb(0, 0, 0));
            win::SetTextColor(dc, win::rgb(255, 255, 255));
            let wide = win::wide(text);
            let mut rect = win::RECT {
                left: 0,
                top: 0,
                right: self.w,
                bottom: self.h,
            };
            win::DrawTextW(
                dc,
                wide.as_ptr(),
                -1,
                &mut rect,
                win::DT_CENTER | win::DT_VCENTER | win::DT_SINGLELINE | win::DT_NOPREFIX,
            );

            win::SelectObject(dc, old_font);
            win::SelectObject(dc, old);
            win::DeleteObject(font);
            win::DeleteDC(dc);
            win::ReleaseDC(core::ptr::null_mut(), screen);

            let mut cov = vec![0u8; (self.w * self.h) as usize];
            for (i, slot) in cov.iter_mut().enumerate() {
                *slot = *mask.bits.add(i * 4); // red channel of the white-on-black mask
            }
            cov
        }
    }

    pub fn premultiply(&self) {
        if !self.is_valid() {
            return;
        }
        unsafe {
            let n = (self.w * self.h * 4) as usize;
            for i in (0..n).step_by(4) {
                let a = *self.bits.add(i + 3) as u32;
                if a == 0 {
                    *self.bits.add(i) = 0;
                    *self.bits.add(i + 1) = 0;
                    *self.bits.add(i + 2) = 0;
                } else if a < 255 {
                    for c in 0..3 {
                        let p = self.bits.add(i + c);
                        *p = ((*p as u32 * a) / 255) as u8;
                    }
                }
            }
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            if !self.bitmap.is_null() {
                win::DeleteObject(self.bitmap);
            }
        }
    }
}

#[inline]
pub fn make_font(pt: f32, dpi: u32, bold: bool) -> win::HFONT {
    let px = (pt * dpi as f32 / 72.0).round().max(1.0) as i32;
    let face = win::wide("Segoe UI");
    unsafe {
        win::CreateFontW(
            -px,
            0,
            0,
            0,
            if bold { win::FW_BOLD } else { win::FW_NORMAL },
            0,
            0,
            0,
            win::DEFAULT_CHARSET,
            win::OUT_TT_PRECIS,
            win::CLIP_DEFAULT_PRECIS,
            win::ANTIALIASED_QUALITY,
            win::DEFAULT_PITCH,
            face.as_ptr(),
        )
    }
}

#[inline]
pub fn draw_text(hdc: win::HDC, text: &str, rect: &mut win::RECT, font: win::HFONT, color: u32, flags: u32) {
    unsafe {
        let old = win::SelectObject(hdc, font);
        win::SetTextColor(hdc, color);
        win::DrawTextW(hdc, win::wide(text).as_ptr(), -1, rect, flags);
        win::SelectObject(hdc, old);
    }
}

/// Anti-aliased coverage of a rounded rectangle at pixel centre (x+0.5, y+0.5).
#[inline]
pub fn rounded_coverage(x: i32, y: i32, w: i32, h: i32, r: f32) -> u8 {
    let px = x as f32 + 0.5;
    let py = y as f32 + 0.5;
    let cx = px.clamp(r, w as f32 - r);
    let cy = py.clamp(r, h as f32 - r);
    let dx = px - cx;
    let dy = py - cy;
    let d = (dx * dx + dy * dy).sqrt();
    ((r + 0.5 - d) * 255.0).clamp(0.0, 255.0) as u8
}

#[inline]
pub fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}

/// Source-over blend with straight (non-premultiplied) alpha.
#[inline]
pub fn blend(dst: &mut [u8; 4], src: [u8; 3], alpha: u8) {
    if alpha == 0 {
        return;
    }
    if alpha == 255 {
        *dst = [src[0], src[1], src[2], 255];
        return;
    }
    let a = alpha as f32 / 255.0;
    let inv = 1.0 - a;
    dst[0] = (src[0] as f32 * a + dst[0] as f32 * inv) as u8;
    dst[1] = (src[1] as f32 * a + dst[1] as f32 * inv) as u8;
    dst[2] = (src[2] as f32 * a + dst[2] as f32 * inv) as u8;
    dst[3] = (255.0 * a + dst[3] as f32 * inv) as u8;
}
