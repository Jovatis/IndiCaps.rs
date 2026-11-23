#![allow(non_snake_case, non_camel_case_types, dead_code)]

//! Minimal Win32 surface. No external crate: only the DLLs already present in Windows.

use core::ffi::c_void;

pub type HWND = *mut c_void;
pub type HDC = *mut c_void;
pub type HBITMAP = *mut c_void;
pub type HICON = *mut c_void;
pub type HBRUSH = *mut c_void;
pub type HPEN = *mut c_void;
pub type HFONT = *mut c_void;
pub type HRGN = *mut c_void;
pub type HHOOK = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HMENU = *mut c_void;
pub type HKEY = *mut c_void;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type COLORREF = u32;
pub type WndProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
pub type HookProc = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

pub const TRUE: i32 = 1;
pub const FALSE: i32 = 0;

pub const WM_NULL: u32 = 0x0000;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_ACTIVATEAPP: u32 = 0x001C;
pub const WM_NCCREATE: u32 = 0x0081;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_SYSKEYUP: u32 = 0x0105;
pub const WM_TIMER: u32 = 0x0113;
pub const WM_SETCURSOR: u32 = 0x0020;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_LBUTTONDBLCLK: u32 = 0x0203;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_MOUSELEAVE: u32 = 0x02A3;
pub const WM_APP_TRAY: u32 = 0x8000;

pub const VK_CAPITAL: i32 = 0x14;
pub const VK_NUMLOCK: i32 = 0x90;
pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
pub const VK_MENU: i32 = 0x12;
pub const VK_LWIN: i32 = 0x5B;
pub const VK_RWIN: i32 = 0x5C;
pub const WH_KEYBOARD_LL: i32 = 13;
pub const VK_ESCAPE: i32 = 0x1B;

pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
pub const WS_EX_TRANSPARENT: u32 = 0x0000_0020;
pub const WS_EX_TOPMOST: u32 = 0x0000_0008;
pub const WS_EX_LAYERED: u32 = 0x0008_0000;
pub const WS_EX_NOACTIVATE: u32 = 0x0800_0000;

pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_SHOWWINDOW: u32 = 0x0040;

pub const HWND_TOPMOST: HWND = -1isize as HWND;
pub const HWND_MESSAGE: HWND = -3isize as HWND;
pub const IDC_ARROW: *const u16 = 32512isize as *const u16;

pub const AC_SRC_OVER: u8 = 0;
pub const AC_SRC_ALPHA: u8 = 1;
pub const ULW_ALPHA: u32 = 2;

pub const SM_XWORKAREA: i32 = 76;
pub const SM_YWORKAREA: i32 = 77;
pub const SM_CXWORKAREA: i32 = 78;
pub const SM_CYWORKAREA: i32 = 79;

pub const GWLP_USERDATA: i32 = -21;
pub const GWLP_ID: i32 = -12;

pub const MF_STRING: u32 = 0;
pub const TPM_RIGHTBUTTON: u32 = 0x0002;
pub const TPM_RETURNCMD: u32 = 0x0100;

pub const NIM_ADD: u32 = 0;
pub const NIM_MODIFY: u32 = 1;
pub const NIM_DELETE: u32 = 2;
pub const NIF_MESSAGE: u32 = 0x01;
pub const NIF_ICON: u32 = 0x02;
pub const NIF_TIP: u32 = 0x04;

pub const KEY_QUERY_VALUE: u32 = 0x0001;
pub const KEY_SET_VALUE: u32 = 0x0002;
pub const REG_SZ: u32 = 1;
pub const HKEY_CURRENT_USER: HKEY = 0x8000_0001isize as HKEY;

pub const TRANSPARENT: i32 = 1;
pub const OPAQUE: i32 = 2;
pub const DT_LEFT: u32 = 0x0000_0000;
pub const DT_CENTER: u32 = 0x0000_0001;
pub const DT_VCENTER: u32 = 0x0000_0004;
pub const DT_SINGLELINE: u32 = 0x0000_0020;
pub const DT_NOPREFIX: u32 = 0x0000_0800;
pub const PS_SOLID: i32 = 0;
pub const SRCCOPY: u32 = 0x00CC_0020;
pub const NULL_BRUSH: i32 = 5;
pub const FW_NORMAL: i32 = 400;
pub const FW_BOLD: i32 = 700;
pub const DEFAULT_CHARSET: u32 = 1;
pub const OUT_TT_PRECIS: u32 = 4;
pub const CLIP_DEFAULT_PRECIS: u32 = 0;
pub const ANTIALIASED_QUALITY: u32 = 4;
pub const DEFAULT_PITCH: u32 = 8;
pub const TME_LEAVE: u32 = 0x0000_0002;
pub const SW_SHOW: i32 = 5;
pub const SW_HIDE: i32 = 0;
pub const ERROR_ALREADY_EXISTS: u32 = 183;

#[repr(C)]
#[derive(Default)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Default)]
pub struct SIZE {
    pub cx: i32,
    pub cy: i32,
}

#[repr(C)]
#[derive(Default)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: u32,
    pub pt: POINT,
    #[cfg(target_pointer_width = "64")]
    pub lPrivate: u32,
}

impl Default for MSG {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: i32,
    pub rcPaint: RECT,
    pub fRestore: i32,
    pub fIncUpdate: i32,
    pub rgbReserved: [u8; 32],
}

impl Default for PAINTSTRUCT {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: u32,
    pub style: u32,
    pub lpfnWndProc: Option<WndProc>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: *mut c_void,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
    pub hIconSm: HICON,
}

impl Default for WNDCLASSEXW {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

#[repr(C)]
pub struct CREATESTRUCTW {
    pub lpCreateParams: *mut c_void,
    pub hInstance: HINSTANCE,
    pub hMenu: HMENU,
    pub hwndParent: HWND,
    pub x: i32,
    pub y: i32,
    pub cx: i32,
    pub cy: i32,
    pub style: i32,
    pub lpszName: *const u16,
    pub lpszClass: *const u16,
    pub dwExStyle: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct KBDLLHOOKSTRUCT {
    pub vkCode: u32,
    pub scanCode: u32,
    pub flags: u32,
    pub time: u32,
    pub dwExtraInfo: usize,
}

#[repr(C)]
pub struct BLENDFUNCTION {
    pub BlendOp: u8,
    pub BlendFlags: u8,
    pub SourceConstantAlpha: u8,
    pub AlphaFormat: u8,
}

#[repr(C)]
#[derive(Default)]
pub struct BITMAPINFOHEADER {
    pub biSize: u32,
    pub biWidth: i32,
    pub biHeight: i32,
    pub biPlanes: u16,
    pub biBitCount: u16,
    pub biCompression: u32,
    pub biSizeImage: u32,
    pub biXPelsPerMeter: i32,
    pub biYPelsPerMeter: i32,
    pub biClrUsed: u32,
    pub biClrImportant: u32,
}

#[repr(C)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [u32; 1],
}

impl BITMAPINFO {
    pub fn new(width: i32, height: i32) -> Self {
        // BI_RGB ignores biSizeImage, but a top-down DIB uses a negative height;
        // storing that as u32 would overflow. Keep the magnitude instead.
        Self {
            bmiHeader: BITMAPINFOHEADER {
                biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: height, // negative => top-down, rows ordered top to bottom
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0, // BI_RGB
                biSizeImage: width.unsigned_abs() * height.unsigned_abs() * 4,
                ..Default::default()
            },
            bmiColors: [0],
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct ICONINFO {
    pub fIcon: i32,
    pub xHotspot: u32,
    pub yHotspot: u32,
    pub hbmMask: HBITMAP,
    pub hbmColor: HBITMAP,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: u32,
    pub hWnd: HWND,
    pub uID: u32,
    pub uFlags: u32,
    pub uCallbackMessage: u32,
    pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: u32,
    pub dwStateMask: u32,
    pub szInfo: [u16; 256],
    pub uTimeoutOrVersion: u32,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: u32,
    pub guidItem: GUID,
    pub hBalloonIcon: HICON,
}

impl Default for NOTIFYICONDATAW {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct LOGFONTW {
    pub lfHeight: i32,
    pub lfWidth: i32,
    pub lfEscapement: i32,
    pub lfOrientation: i32,
    pub lfWeight: i32,
    pub lfItalic: u8,
    pub lfUnderline: u8,
    pub lfStrikeOut: u8,
    pub lfCharSet: u8,
    pub lfOutPrecision: u8,
    pub lfClipPrecision: u8,
    pub lfQuality: u8,
    pub lfPitchAndFamily: u8,
    pub lfFaceName: [u16; 32],
}

#[repr(C)]
#[derive(Default)]
pub struct SYSTEMTIME {
    pub wYear: u16,
    pub wMonth: u16,
    pub wDayOfWeek: u16,
    pub wDay: u16,
    pub wHour: u16,
    pub wMinute: u16,
    pub wSecond: u16,
    pub wMilliseconds: u16,
}

#[repr(C)]
#[derive(Default)]
pub struct TRACKMOUSEEVENT {
    pub cbSize: u32,
    pub dwFlags: u32,
    pub hwndTrack: HWND,
    pub dwHoverTime: u32,
}

#[inline]
pub fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    r as COLORREF | ((g as COLORREF) << 8) | ((b as COLORREF) << 16)
}

#[inline]
pub fn wide(s: &str) -> Vec<u16> {
    use core::iter::once;
    s.encode_utf16().chain(once(0)).collect()
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
    pub fn GetLastError() -> u32;
    pub fn CreateMutexW(attrs: *const c_void, initial: i32, name: *const u16) -> *mut c_void;
    pub fn GetLocalTime(lpTime: *mut SYSTEMTIME);
    pub fn GetSystemDirectoryW(lpBuffer: *mut u16, uSize: u32) -> u32;
    pub fn GetComputerNameW(lpBuffer: *mut u16, nSize: *mut u32) -> i32;
    pub fn GetVolumeInformationW(
        lpRootPathName: *const u16,
        lpVolumeNameBuffer: *mut u16,
        nVolumeNameSize: u32,
        lpVolumeSerialNumber: *mut u32,
        lpMaximumComponentLength: *mut u32,
        lpFileSystemFlags: *mut u32,
        lpFileSystemNameBuffer: *mut u16,
        nFileSystemNameSize: u32,
    ) -> i32;
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> u16;
    pub fn CreateWindowExW(
        dwExStyle: u32,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        hwndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: *const c_void,
    ) -> HWND;
    pub fn DefWindowProcW(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn DestroyWindow(hwnd: HWND) -> i32;
    pub fn ShowWindow(hwnd: HWND, cmd: i32) -> i32;
    pub fn UpdateWindow(hwnd: HWND) -> i32;
    pub fn InvalidateRect(hwnd: HWND, rect: *const RECT, erase: i32) -> i32;
    pub fn BeginPaint(hwnd: HWND, paint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hwnd: HWND, paint: *const PAINTSTRUCT) -> i32;
    pub fn GetMessageW(msg: *mut MSG, hwnd: HWND, min: u32, max: u32) -> i32;
    pub fn TranslateMessage(msg: *const MSG) -> i32;
    pub fn DispatchMessageW(msg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(code: i32);
    pub fn PostMessageW(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> i32;
    pub fn SetWindowLongPtrW(hwnd: HWND, index: i32, new: isize) -> isize;
    pub fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;
    pub fn SetWindowPos(
        hwnd: HWND,
        after: HWND,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    pub fn SetWindowRgn(hwnd: HWND, rgn: HRGN, redraw: i32) -> i32;
    pub fn GetSystemMetrics(index: i32) -> i32;
    pub fn SetWindowsHookExW(id: i32, lpfn: HookProc, hmod: HINSTANCE, tid: u32) -> HHOOK;
    pub fn UnhookWindowsHookEx(hhk: HHOOK) -> i32;
    pub fn CallNextHookEx(hhk: HHOOK, code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn GetKeyState(key: i32) -> i16;
    pub fn LoadCursorW(hinstance: HINSTANCE, name: *const u16) -> HICON;
    pub fn SetCursor(cursor: HICON) -> HICON;
    pub fn SetCapture(hwnd: HWND) -> HWND;
    pub fn ReleaseCapture() -> i32;
    pub fn GetDC(hwnd: HWND) -> HDC;
    pub fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    pub fn UpdateLayeredWindow(
        hwnd: HWND,
        hdcDst: HDC,
        pptDst: *const POINT,
        psize: *const SIZE,
        hdcSrc: HDC,
        pptSrc: *const POINT,
        crKey: u32,
        pblend: *const BLENDFUNCTION,
        dwFlags: u32,
    ) -> i32;
    pub fn SetTimer(hwnd: HWND, id: usize, elapse: u32, proc: *const c_void) -> usize;
    pub fn KillTimer(hwnd: HWND, id: usize) -> i32;
    pub fn GetDpiForSystem() -> u32;
    pub fn GetCursorPos(pt: *mut POINT) -> i32;
    pub fn SetForegroundWindow(hwnd: HWND) -> i32;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn AppendMenuW(hmenu: HMENU, flags: u32, id: usize, data: *const u16) -> i32;
    pub fn TrackPopupMenuEx(
        hmenu: HMENU,
        flags: u32,
        x: i32,
        y: i32,
        hwnd: HWND,
        pt: *const c_void,
    ) -> i32;
    pub fn DestroyMenu(hmenu: HMENU) -> i32;
    pub fn CreateIconIndirect(info: *const ICONINFO) -> HICON;
    pub fn LoadIconW(hinstance: HINSTANCE, name: *const u16) -> HICON;
    pub fn DrawIconEx(
        hdc: HDC,
        x: i32,
        y: i32,
        hicon: HICON,
        cx: i32,
        cy: i32,
        step: u32,
        flicker_free: HBRUSH,
        flags: u32,
    ) -> i32;
    pub fn DestroyIcon(hicon: HICON) -> i32;
    pub fn TrackMouseEvent(lpEventTrack: *mut TRACKMOUSEEVENT) -> i32;
    pub fn SetProcessDpiAwarenessContext(ctx: isize) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn CreateCompatibleBitmap(hdc: HDC, width: i32, height: i32) -> HBITMAP;
    pub fn BitBlt(
        hdc: HDC,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        src: HDC,
        sx: i32,
        sy: i32,
        op: u32,
    ) -> i32;
    pub fn GetStockObject(index: i32) -> *mut c_void;
    pub fn DeleteDC(hdc: HDC) -> i32;
    pub fn SelectObject(hdc: HDC, obj: *mut c_void) -> *mut c_void;
    pub fn DeleteObject(obj: *mut c_void) -> i32;
    pub fn CreateDIBSection(
        hdc: HDC,
        bmi: *const BITMAPINFO,
        usage: u32,
        bits: *mut *mut c_void,
        section: *mut c_void,
        offset: u32,
    ) -> HBITMAP;
    pub fn CreateBitmap(width: i32, height: i32, planes: u32, bpp: u32, bits: *const c_void)
        -> HBITMAP;
    pub fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    pub fn CreatePen(style: i32, width: i32, color: COLORREF) -> HPEN;
    pub fn RoundRect(hdc: HDC, x1: i32, y1: i32, x2: i32, y2: i32, w: i32, h: i32) -> i32;
    pub fn Rectangle(hdc: HDC, x1: i32, y1: i32, x2: i32, y2: i32) -> i32;
    pub fn FillRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> i32;
    pub fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    pub fn SetBkColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn DrawTextW(
        hdc: HDC,
        text: *const u16,
        count: i32,
        rect: *mut RECT,
        flags: u32,
    ) -> i32;
    pub fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strikeout: u32,
        charset: u32,
        out_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> HFONT;
    pub fn CreateRoundRectRgn(x1: i32, y1: i32, x2: i32, y2: i32, w: i32, h: i32) -> HRGN;
    pub fn MoveToEx(hdc: HDC, x: i32, y: i32, pt: *mut POINT) -> i32;
    pub fn LineTo(hdc: HDC, x: i32, y: i32) -> i32;
    pub fn Ellipse(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(message: u32, data: *const NOTIFYICONDATAW) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegOpenKeyExW(
        key: HKEY,
        subKey: *const u16,
        options: u32,
        rights: u32,
        result: *mut HKEY,
    ) -> i32;
    pub fn RegSetValueExW(
        key: HKEY,
        name: *const u16,
        reserved: u32,
        ty: u32,
        data: *const u8,
        len: u32,
    ) -> i32;
    pub fn RegDeleteValueW(key: HKEY, name: *const u16) -> i32;
    pub fn RegCloseKey(key: HKEY) -> i32;
}

#[link(name = "winmm")]
extern "system" {
    pub fn PlaySoundW(sound: *const u16, hmod: HINSTANCE, flags: u32) -> i32;
}
