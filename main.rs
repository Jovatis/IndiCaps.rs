#![windows_subsystem = "windows"]
// Win32 identifiers and COLORREF byte grouping deliberately mirror the native
// API (`HWND`, `0x00BBGGRR`), so Rust's naming/formatting lints are noise here.
#![allow(clippy::upper_case_acronyms, clippy::unusual_byte_groupings)]
// Drawing helpers map directly to GDI/GDI+ calls with wide signatures.
#![allow(clippy::too_many_arguments)]

mod draw;
mod gfx;
mod settings;
mod win;

use core::sync::atomic::{AtomicBool, Ordering};
use draw::{blend, draw_text, make_font, mix, rounded_coverage, Surface};
use gfx::{argb, Gfx};
use settings::{text, Lang, Settings, LANG_COUNT, LANGS};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use win::*;

const CLASS_NAME: &str = "IndiCapsNative";
const MUTEX_NAME: &str = "IndiCaps_SingleInstance";

const SND_ASYNC: u32 = 0x0001;
const SND_MEMORY: u32 = 0x0004;
const SND_ON: &[u8] = include_bytes!("../assets/capson.wav");
const SND_OFF: &[u8] = include_bytes!("../assets/capsoff.wav");

/// Returns a PCM WAV whose 16-bit samples are scaled by `volume` percent.
/// The original header and chunk layout are preserved byte for byte.
fn scaled_wav(source: &[u8], volume: u8) -> Vec<u8> {
    let mut out = source.to_vec();
    if volume == 100 {
        return out;
    }
    let Some(chunk) = source.windows(4).position(|w| w == b"data") else {
        return out;
    };
    if chunk + 8 > source.len() {
        return out;
    }
    let size = u32::from_le_bytes([
        source[chunk + 4],
        source[chunk + 5],
        source[chunk + 6],
        source[chunk + 7],
    ]) as usize;
    let start = chunk + 8;
    let end = start.saturating_add(size).min(source.len()).min(out.len());
    let mut i = start;
    while i + 1 < end {
        let sample = i16::from_le_bytes([source[i], source[i + 1]]);
        let scaled = ((sample as i32 * volume as i32 / 100)
            .clamp(i16::MIN as i32, i16::MAX as i32)) as i16;
        out[i..i + 2].copy_from_slice(&scaled.to_le_bytes());
        i += 2;
    }
    out
}

const KIND_MSG: u8 = 0;
const KIND_OVERLAY: u8 = 1;
const KIND_PANEL: u8 = 2;
const KIND_POS: u8 = 3;
const KIND_GHOST: u8 = 4;

const WM_APP_CAPS: u32 = WM_APP_TRAY + 1;
const WM_APP_NUM: u32 = WM_APP_TRAY + 2;
const WM_APP_HOTKEY: u32 = WM_APP_TRAY + 3;
const WM_APP_REFRESH: u32 = WM_APP_TRAY + 4;
const WM_APP_HOTKEY_STORED: u32 = WM_APP_TRAY + 5;
const TIMER_OVERLAY: usize = 1;
const TIMER_PANEL: usize = 2;
const TIMER_STATS: usize = 3;

const PANEL_W: i32 = 380;
const PANEL_H: i32 = 500;

// Header: official logo + IndiCaps title on one line, with the category
// ("Settings") left-aligned underneath the whole group, as in the v2 mockup.
const IDI_LOGO: usize = 1;
const HEADER_LOGO_X: i32 = 19;
const HEADER_LOGO_Y: i32 = 20;
const HEADER_LOGO_SIZE: i32 = 30;
const HEADER_TITLE_X: i32 = 58;
const HEADER_TITLE_Y: i32 = 19;
const HEADER_TITLE_W: i32 = 220;
const HEADER_TITLE_H: i32 = 34;
const HEADER_SUBTITLE_X: i32 = 27;
const HEADER_SUBTITLE_Y: i32 = 57;
const HEADER_SUBTITLE_W: i32 = 220;
const HEADER_SUBTITLE_H: i32 = 26;

// Header: burger category selector + close.
const BURGER_X: i32 = 298;
const BURGER_Y: i32 = 24;
const BURGER_W: i32 = 28;
const BURGER_H: i32 = 28;
const CLOSE_X: i32 = 334;
const CLOSE_Y: i32 = 24;
const CLOSE_W: i32 = 28;
const CLOSE_H: i32 = 28;

// Dropdown: header is a separate band so it never overlaps the first row.
const MENU_X: i32 = 160;
const MENU_Y: i32 = 62;
const MENU_W: i32 = 196;
const MENU_PAD: i32 = 8;
const MENU_HEADER_H: i32 = 28;
const MENU_ROW_H: i32 = 40;
const CATEGORY_COUNT: usize = 4;
const MENU_H: i32 =
    MENU_PAD * 2 + MENU_HEADER_H + CATEGORY_COUNT as i32 * MENU_ROW_H;
const CATEGORY_KEYS: [&str; CATEGORY_COUNT] = [
    "tab_settings", "tab_stats", "tab_hotkeys", "tab_display",
];

// Settings view: one full-width language selector opening a 2-column list.
const LANG_LABEL_Y: i32 = 96;
const LANG_BTN_X: i32 = 24;
const LANG_BTN_Y: i32 = 122;
const LANG_BTN_W: i32 = 332;
const LANG_BTN_H: i32 = 32;
const LANG_MENU_X: i32 = 24;
const LANG_MENU_Y: i32 = 158;
const LANG_MENU_W: i32 = 332;
const LANG_MENU_PAD: i32 = 6;
const LANG_MENU_HEADER_H: i32 = 24;
const LANG_ROW_H: i32 = 27;
const LANG_COLS: usize = 2;
const LANG_ROWS: usize = LANG_COUNT.div_ceil(LANG_COLS);
const LANG_MENU_H: i32 =
    LANG_MENU_PAD * 2 + LANG_MENU_HEADER_H + LANG_ROWS as i32 * LANG_ROW_H;
const OPT_ROW_Y: i32 = 188;
const OPT_ROW_STEP: i32 = 44;
const OPT_ROW_H: i32 = 36;
const OPT_COUNT: usize = 5;
const OPT_KEYS: [&str; OPT_COUNT] = [
    "opt_indicator",
    "opt_num",
    "opt_sound",
    "opt_animation",
    "opt_startup",
];

// Display view: four snapped sliders.
const SLIDER_X: i32 = 24;
const SLIDER_W: i32 = 248;
const SLIDER_TRACK_H: i32 = 10;
const SLIDER_VAL_X: i32 = 286;
const SLIDER_VAL_W: i32 = 70;
const SLIDER_HIT_Y_OFF: i32 = -14;
const SLIDER_HIT_H: i32 = 36;
const DURATION_TRACK_Y: i32 = 150;
const PANEL_OP_TRACK_Y: i32 = 216;
const OVERLAY_OP_TRACK_Y: i32 = 282;
const SOUND_VOLUME_TRACK_Y: i32 = 348;
const DURATIONS: [u32; 4] = [800, 1500, 3000, 0];
const OPACITY_STEPS: [u8; 9] = [20, 30, 40, 50, 60, 70, 80, 90, 100];
const SOUND_STEPS: [u8; 11] = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
const SNAP_COUNT: usize = 7;
/// User scale: 100 = fully visible, 0 = invisible. 80% black dim => alpha 204.
const POS_DIM_ALPHA: u8 = 204;

// Panel hit-test identifiers.
const ID_NONE: i32 = 0;
const ID_QUIT: i32 = 3;
const ID_OPT_BASE: i32 = 4;
const ID_CLOSE: i32 = 9;
const ID_MENU: i32 = 10;
const ID_CATEGORY_BASE: i32 = 11;
const ID_RESET: i32 = 16;
const ID_HOTKEY: i32 = 17;
const ID_DURATION_SLIDER: i32 = 18;
const ID_PANEL_SLIDER: i32 = 19;
const ID_OVERLAY_SLIDER: i32 = 20;
const ID_POSITION: i32 = 21;
// Language dropdown uses a separate range so the existing ids stay stable.
const ID_LANG_SELECT: i32 = 22;
const ID_LANG_BASE: i32 = 23;
const ID_SOUND_SLIDER: i32 = 41;
const ID_RESET_NO: i32 = 42;
const ID_RESET_YES: i32 = 43;

const C_BG: u32 = 0x1414_14;
const C_TRACK: u32 = 0x3030_36;
const C_BORDER: u32 = 0x2626_2C;
const C_HOVER: u32 = 0x2626_2C;
const C_TEXT: u32 = 0xFFFF_FF;
const C_SECONDARY: u32 = 0x9F9F_A5;
const C_ACCENT: u32 = 0x84E0_5E; // COLORREF for RGB(94, 224, 132)
const C_SELECT: u32 = 0x0024_382A; // dark green tint behind selected text

#[repr(C)]
struct CreateInfo {
    app: *mut App,
    kind: u8,
}

static mut APP_PTR: *mut App = core::ptr::null_mut();
/// Guards against re-entrant `&mut App` aliasing while a nested modal loop runs.
static HOOK_BUSY: AtomicBool = AtomicBool::new(false);

#[inline]
fn lo(v: LPARAM) -> i32 {
    (v & 0xFFFF) as i16 as i32
}

#[inline]
fn hi(v: LPARAM) -> i32 {
    ((v >> 16) & 0xFFFF) as i16 as i32
}

#[inline]
fn inside(x: i32, y: i32, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

/// Screen rectangle of one language cell in the dropdown grid.
fn lang_cell_rect(i: usize) -> (i32, i32, i32, i32) {
    let col = i % LANG_COLS;
    let row = i / LANG_COLS;
    let cell_w = (LANG_MENU_W - LANG_MENU_PAD * 2) / LANG_COLS as i32;
    let x = LANG_MENU_X + LANG_MENU_PAD + col as i32 * cell_w;
    let y = LANG_MENU_Y + LANG_MENU_PAD + LANG_MENU_HEADER_H + row as i32 * LANG_ROW_H;
    (x, y, cell_w, LANG_ROW_H)
}

fn work_area() -> (i32, i32, i32, i32) {
    unsafe {
        let left = GetSystemMetrics(SM_XWORKAREA);
        let top = GetSystemMetrics(SM_YWORKAREA);
        (left, top, left + GetSystemMetrics(SM_CXWORKAREA), top + GetSystemMetrics(SM_CYWORKAREA))
    }
}

fn panel_pos() -> (i32, i32) {
    let (_, _, right, bottom) = work_area();
    (right - PANEL_W - 8, bottom - PANEL_H - 8)
}

fn overlay_size(dpi: u32) -> (i32, i32) {
    let w = ((210u32 * dpi / 96).min(683)) as i32;
    let h = ((w as f32 * 226.0 / 683.0).round().max(1.0)) as i32;
    (w, h)
}

/// Seven snap anchors, expressed as the top-left corner of the indicator.
fn snap_anchors(w: i32, h: i32) -> [(i32, i32); SNAP_COUNT] {
    let (left, top, right, bottom) = work_area();
    let margin = 36;
    let mid_x = left + (right - left - w) / 2;
    [
        (right - w - margin, bottom - h - margin), // 1: bottom-right (default)
        (left + margin, bottom - h - margin),      // 2: bottom-left
        (right - w - margin, top + margin),        // 3: top-right
        (left + margin, top + margin),             // 4: top-left
        (mid_x, top + margin),                     // 5: top-center
        (mid_x, top + (bottom - top - h) / 2),     // 6: center
        (mid_x, bottom - h - margin),              // 7: bottom-center
    ]
}

/// Centres a text card in the free gap between two snap cards.
fn gap_center(after_bottom: i32, before_top: i32, height: i32) -> i32 {
    let room = before_top - after_bottom;
    if room >= height + 16 {
        after_bottom + (room - height) / 2
    } else {
        after_bottom + (room - height).max(0) / 2
    }
}

fn overlay_xy(w: i32, h: i32, settings: &Settings) -> (i32, i32) {
    let (left, top, right, bottom) = work_area();
    let anchors = snap_anchors(w, h);
    let (mut x, mut y) = if settings.overlay_x < 0 || settings.overlay_y < 0 {
        anchors[0]
    } else {
        (settings.overlay_x, settings.overlay_y)
    };
    x = x.clamp(left, (right - w).max(left));
    y = y.clamp(top, (bottom - h).max(top));
    (x, y)
}

fn snap_index(x: i32, y: i32, w: i32, h: i32) -> usize {
    let anchors = snap_anchors(w, h);
    let cx = x + w / 2;
    let cy = y + h / 2;
    let mut best = 0usize;
    let mut best_d = i64::MAX;
    for (i, (ax, ay)) in anchors.iter().enumerate() {
        let dx = (ax + w / 2 - cx) as i64;
        let dy = (ay + h / 2 - cy) as i64;
        let d = dx * dx + dy * dy;
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

/// Slider kinds used by both hit-testing and rendering.
const SLIDER_DURATION: u8 = 1;
const SLIDER_PANEL_OP: u8 = 2;
const SLIDER_OVERLAY_OP: u8 = 3;
const SLIDER_SOUND: u8 = 4;

fn slider_kind(id: i32) -> u8 {
    match id {
        ID_DURATION_SLIDER => SLIDER_DURATION,
        ID_PANEL_SLIDER => SLIDER_PANEL_OP,
        ID_OVERLAY_SLIDER => SLIDER_OVERLAY_OP,
        ID_SOUND_SLIDER => SLIDER_SOUND,
        _ => 0,
    }
}

fn slider_steps(kind: u8) -> usize {
    match kind {
        SLIDER_DURATION => DURATIONS.len(),
        SLIDER_SOUND => SOUND_STEPS.len(),
        _ => OPACITY_STEPS.len(),
    }
}

fn slider_track_y(kind: u8) -> i32 {
    match kind {
        SLIDER_DURATION => DURATION_TRACK_Y,
        SLIDER_PANEL_OP => PANEL_OP_TRACK_Y,
        SLIDER_OVERLAY_OP => OVERLAY_OP_TRACK_Y,
        _ => SOUND_VOLUME_TRACK_Y,
    }
}

fn slider_index(settings: &Settings, kind: u8) -> usize {
    match kind {
        SLIDER_DURATION => DURATIONS
            .iter()
            .position(|&v| v == settings.overlay_ms)
            .unwrap_or(1),
        SLIDER_PANEL_OP => OPACITY_STEPS
            .iter()
            .position(|&v| v == settings.panel_opacity)
            .unwrap_or(OPACITY_STEPS.len() - 1),
        SLIDER_OVERLAY_OP => OPACITY_STEPS
            .iter()
            .position(|&v| v == settings.overlay_opacity)
            .unwrap_or(OPACITY_STEPS.len() - 1),
        _ => SOUND_STEPS
            .iter()
            .position(|&v| v == settings.sound_volume)
            .unwrap_or(SOUND_STEPS.len() - 1),
    }
}

fn slider_value_text(settings: &Settings, lang: Lang, kind: u8) -> String {
    let idx = slider_index(settings, kind);
    match kind {
        SLIDER_DURATION => text(lang, &format!("duration_{}", idx + 1)).to_string(),
        SLIDER_SOUND => format!("{} %", SOUND_STEPS[idx]),
        _ => format!("{} %", OPACITY_STEPS[idx]),
    }
}

fn slider_index_from_x(x: i32, kind: u8) -> usize {
    let steps = slider_steps(kind);
    let t = ((x - SLIDER_X) as f32 / SLIDER_W as f32).clamp(0.0, 1.0);
    (t * (steps - 1) as f32).round() as usize
}

fn slider_values_differ(settings: &Settings, kind: u8, idx: usize) -> bool {
    match kind {
        SLIDER_DURATION => idx < DURATIONS.len() && DURATIONS[idx] != settings.overlay_ms,
        SLIDER_PANEL_OP => {
            idx < OPACITY_STEPS.len() && OPACITY_STEPS[idx] != settings.panel_opacity
        }
        SLIDER_OVERLAY_OP => {
            idx < OPACITY_STEPS.len() && OPACITY_STEPS[idx] != settings.overlay_opacity
        }
        _ => idx < SOUND_STEPS.len() && SOUND_STEPS[idx] != settings.sound_volume,
    }
}

fn slider_apply(settings: &mut Settings, kind: u8, idx: usize) {
    if !slider_values_differ(settings, kind, idx) {
        return;
    }
    match kind {
        SLIDER_DURATION => {
            if idx < DURATIONS.len() {
                settings.overlay_ms = DURATIONS[idx];
            }
        }
        SLIDER_PANEL_OP => {
            if idx < OPACITY_STEPS.len() {
                settings.panel_opacity = OPACITY_STEPS[idx];
            }
        }
        SLIDER_OVERLAY_OP => {
            if idx < OPACITY_STEPS.len() {
                settings.overlay_opacity = OPACITY_STEPS[idx];
            }
        }
        _ => {
            if idx < SOUND_STEPS.len() {
                settings.sound_volume = SOUND_STEPS[idx];
            }
        }
    }
}

/// Draws one snapped slider with its value bubble (GDI+ path).
unsafe fn paint_slider(
    canvas: &mut gfx::Canvas<'_>,
    y: i32,
    kind: u8,
    idx: usize,
    value: &str,
    active: bool,
) {
    let x = SLIDER_X as f32;
    let w = SLIDER_W as f32;
    let steps = slider_steps(kind);
    let usable = w - 18.0;
    let cy = y as f32 + SLIDER_TRACK_H as f32 / 2.0;
    let center = x + 9.0 + usable * idx as f32 / (steps - 1) as f32;

    canvas.round_rect(
        x, y as f32, w, SLIDER_TRACK_H as f32, 5.0,
        argb(C_TRACK), argb(C_BORDER), 1.0,
    );
    if idx > 0 {
        canvas.round_rect(
            x, y as f32, center - x, SLIDER_TRACK_H as f32, 5.0,
            argb(C_ACCENT), argb(C_ACCENT), 0.0,
        );
    }
    for i in 0..steps {
        let tx = x + 9.0 + usable * i as f32 / (steps - 1) as f32;
        let col = if i <= idx { argb(0x0C0C_10) } else { 0x66FF_FFFF };
        canvas.ellipse(tx - 1.5, cy - 1.5, 3.0, 3.0, col, 0, 0.0);
    }
    canvas.ellipse(center - 8.0, cy - 8.0, 18.0, 18.0, 0x4D00_0000, 0, 0.0);
    canvas.ellipse(
        center - 9.0, cy - 9.0, 18.0, 18.0,
        argb(C_TEXT),
        if active { argb(C_ACCENT) } else { argb(C_BORDER) },
        2.0,
    );

    let vy = y as f32 - 11.0;
    let (fill, border) = if active {
        (argb(C_SELECT), argb(C_ACCENT))
    } else {
        (argb(C_TRACK), argb(C_BORDER))
    };
    canvas.round_rect(
        SLIDER_VAL_X as f32, vy, SLIDER_VAL_W as f32, 32.0, 10.0,
        fill, border, 1.0,
    );
    canvas.text(
        value, SLIDER_VAL_X as f32, vy, SLIDER_VAL_W as f32, 32.0, 15.0,
        argb(C_TEXT), true,
    );
}

fn format_duration(ms: u64) -> String {
    let total = ms / 1000;
    if total < 60 {
        format!("{total} s")
    } else if total < 3600 {
        format!("{} min {} s", total / 60, total % 60)
    } else if total < 86_400 {
        format!("{} h {} min", total / 3600, (total % 3600) / 60)
    } else {
        format!("{} j {} h", total / 86_400, (total % 86_400) / 3600)
    }
}

fn format_minutes(secs: u64) -> String {
    format!("{} min", secs / 60)
}

fn stats_since(settings: &Settings) -> String {
    if settings.stats_started == 0 {
        return "—".into();
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    format_minutes(now.saturating_sub(settings.stats_started))
}

unsafe fn modifier_mask() -> u8 {
    let mut m = 0u8;
    if GetKeyState(VK_MENU) < 0 { m |= 1 }
    if GetKeyState(VK_CONTROL) < 0 { m |= 2 }
    if GetKeyState(VK_SHIFT) < 0 { m |= 4 }
    if GetKeyState(VK_LWIN) < 0 || GetKeyState(VK_RWIN) < 0 { m |= 8 }
    m
}

fn modifier_bit(vk: u32) -> Option<u8> {
    match vk {
        0x12 | 0xA4 | 0xA5 => Some(1), // Alt
        0x11 | 0xA2 | 0xA3 => Some(2), // Ctrl
        0x10 | 0xA0 | 0xA1 => Some(4), // Shift
        0x5B | 0x5C => Some(8),        // Win
        _ => None,
    }
}

fn is_modifier(vk: u32) -> bool {
    modifier_bit(vk).is_some()
}

fn is_toggle_key(vk: u32) -> bool {
    matches!(vk as i32, VK_CAPITAL | VK_NUMLOCK)
}

fn key_name(vk: u32) -> String {
    if (0x41..=0x5A).contains(&vk) || (0x30..=0x39).contains(&vk) {
        return (vk as u8 as char).to_string();
    }
    if (0x70..=0x7B).contains(&vk) {
        return format!("F{}", vk - 0x6F);
    }
    match vk {
        0x20 => "Space".into(),
        0x0D => "Enter".into(),
        0x09 => "Tab".into(),
        0x08 => "Backspace".into(),
        0x2E => "Del".into(),
        0x2D => "Insert".into(),
        0x24 => "Home".into(),
        0x23 => "End".into(),
        0x21 => "PgUp".into(),
        0x22 => "PgDn".into(),
        0x25 => "←".into(),
        0x26 => "↑".into(),
        0x27 => "→".into(),
        0x28 => "↓".into(),
        _ => format!("0x{vk:02X}"),
    }
}

fn hotkey_label(settings: &Settings) -> String {
    let lang = settings.language;
    let mut parts: Vec<String> = Vec::new();
    if settings.hotkey_mod & 1 != 0 { parts.push("Alt".into()); }
    if settings.hotkey_mod & 2 != 0 { parts.push(text(lang, "key_ctrl").into()); }
    if settings.hotkey_mod & 4 != 0 {
        parts.push(text(lang, "key_shift").into());
    }
    if settings.hotkey_mod & 8 != 0 { parts.push("Win".into()); }
    parts.push(key_name(settings.hotkey_vk));
    parts.join(" + ")
}

struct Overlay {
    surface: Option<Surface>,
    phase: u8, // 0 idle, 1 fade in, 2 hold, 3 fade out
    count: i32,
    animate: bool,
    caps_on: bool,
    text: String,
    caption_key: &'static str,
    lang: Lang,
    dpi: u32,
    x: i32,
    y: i32,
    opacity: u8,
    hold_ticks: i32,
}

impl Overlay {
    fn new(dpi: u32) -> Self {
        Self {
            surface: None,
            phase: 0,
            count: 0,
            animate: true,
            caps_on: false,
            text: String::new(),
            caption_key: "",
            lang: Lang::En,
            dpi,
            x: 0,
            y: 0,
            opacity: 100,
            hold_ticks: 125,
        }
    }

    fn display(
        &mut self,
        hwnd: HWND,
        caption: &str,
        caption_key: &'static str,
        caps_on: bool,
        animate: bool,
        gfx: Option<&Gfx>,
        settings: &Settings,
    ) {
        let (w, h) = overlay_size(self.dpi);
        if self.surface.as_ref().is_none_or(|s| s.w != w || s.h != h) {
            self.surface = Some(Surface::new(w, h));
        }
        self.text.clear();
        self.text.push_str(caption);
        self.caption_key = caption_key;
        self.lang = settings.language;
        self.caps_on = caps_on;
        self.animate = animate;
        self.opacity = settings.overlay_opacity;
        self.hold_ticks = if settings.overlay_ms == 0 {
            i32::MAX // permanent until the next state change
        } else {
            (settings.overlay_ms / 12).max(1) as i32
        };
        self.render(gfx);

        let (x, y) = overlay_xy(w, h, settings);
        self.x = x;
        self.y = y;
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
        let start = if animate { 0 } else { self.opacity as i32 * 255 / 100 };
        self.set_alpha(hwnd, start);
        self.phase = 1;
        self.count = 0;
    }

    /// Re-renders a visible notification after the UI language changes.
    fn set_language(&mut self, lang: Lang, gfx: Option<&Gfx>) {
        self.lang = lang;
        if self.phase == 0 || self.caption_key.is_empty() {
            return;
        }
        self.text = text(lang, self.caption_key).to_string();
        self.render(gfx);
    }

    fn render(&mut self, gfx: Option<&Gfx>) {
        let Some(surface) = self.surface.as_ref() else { return };
        if !surface.is_valid() {
            return;
        }
        let w = surface.w;
        let h = surface.h;
        let pt = (w as f32 * 0.075).round().max(8.0);

        if let Some(gfx) = gfx {
            unsafe { gfx.render_overlay(surface.bits, w, h, self.caps_on, &self.text, pt, self.lang) };
            surface.premultiply();
            return;
        }

        let accent: [u8; 3] = if self.caps_on { [94, 224, 132] } else { [150, 150, 162] };

        for y in 0..h {
            let t = y as f32 / h.max(1) as f32;
            let base = mix([24, 24, 30], [13, 13, 18], t);
            for x in 0..w {
                let outer = rounded_coverage(x, y, w, h, 10.0);
                if outer == 0 {
                    continue;
                }
                let inner = rounded_coverage(x - 1, y - 1, w - 2, h - 2, 9.0);
                let col = if inner < 250 {
                    mix(base, accent, (255 - inner) as f32 / 255.0)
                } else {
                    base
                };
                surface.put(x, y, [col[0], col[1], col[2], outer]);
            }
        }

        let mask = surface.text_mask(&self.text, pt, self.dpi, false);
        if mask.len() < (w * h) as usize {
            return;
        }
        for y in 0..h {
            for x in 0..w {
                let cov = mask[(y * w + x) as usize];
                if cov == 0 {
                    continue;
                }
                let shadow_alpha = (cov as u32 * 140 / 255) as u8;
                let mut dst = surface.get(x, y + 2);
                blend(&mut dst, [0, 0, 0], shadow_alpha);
                surface.put(x, y + 2, dst);

                let mut dst = surface.get(x, y);
                blend(&mut dst, [255, 255, 255], cov);
                surface.put(x, y, dst);
            }
        }
        surface.premultiply();
    }

    fn set_alpha(&self, hwnd: HWND, alpha: i32) {
        let Some(surface) = self.surface.as_ref() else { return };
        let scaled = alpha * self.opacity as i32 / 100;
        unsafe {
            let screen = GetDC(core::ptr::null_mut());
            let mem = CreateCompatibleDC(screen);
            let old = SelectObject(mem, surface.bitmap);
            let pos = POINT { x: self.x, y: self.y };
            let size = SIZE { cx: surface.w, cy: surface.h };
            let src = POINT { x: 0, y: 0 };
            let blend_fn = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER,
                BlendFlags: 0,
                SourceConstantAlpha: scaled.clamp(0, 255) as u8,
                AlphaFormat: AC_SRC_ALPHA,
            };
            UpdateLayeredWindow(
                hwnd, screen, &pos, &size, mem, &src, 0, &blend_fn, ULW_ALPHA,
            );
            SelectObject(mem, old);
            DeleteDC(mem);
            ReleaseDC(core::ptr::null_mut(), screen);
        }
    }

    /// Returns false when the animation is finished.
    fn tick(&mut self, hwnd: HWND) -> bool {
        match self.phase {
            1 => {
                if self.animate {
                    self.count += 1;
                    self.set_alpha(hwnd, 255 * self.count / 12);
                    if self.count >= 12 {
                        self.count = 0;
                        if self.hold_ticks == i32::MAX {
                            // Permanent overlay: stop the timer once fully visible.
                            return false;
                        }
                        self.phase = 2;
                    }
                } else {
                    self.set_alpha(hwnd, 255);
                    self.count = 0;
                    if self.hold_ticks == i32::MAX {
                        return false;
                    }
                    self.phase = 2;
                }
            }
            2 => {
                if self.hold_ticks == i32::MAX {
                    // Permanent mode: stay until the next toggle replaces us.
                } else {
                    self.count += 1;
                    if self.count >= self.hold_ticks {
                        self.count = 0;
                        self.phase = 3;
                    }
                }
            }
            3 => {
                if self.animate {
                    self.count += 1;
                    self.set_alpha(hwnd, 255 * (18 - self.count) / 18);
                    if self.count >= 18 {
                        self.hide(hwnd);
                        return false;
                    }
                } else {
                    self.hide(hwnd);
                    return false;
                }
            }
            _ => return false,
        }
        true
    }

    fn hide(&mut self, hwnd: HWND) {
        self.phase = 0;
        self.count = 0;
        unsafe { ShowWindow(hwnd, SW_HIDE) };
    }
}

struct Panel {
    visible: bool,
    surface: Option<Surface>,
    knob: [f32; OPT_COUNT],
    target: [f32; OPT_COUNT],
    hover: i32,
    tracking: bool,
    dpi: u32,
    view: u8, // 0 settings, 1 stats, 2 hotkeys, 3 display
    hotkey_capturing: bool,
    reset_confirm: bool,
    menu_open: bool,
    lang_open: bool,
    drag_slider: u8,
}

impl Panel {
    fn new(settings: &Settings, dpi: u32) -> Self {
        let values = [
            settings.indicator,
            settings.num_indicator,
            settings.sound,
            settings.animation,
            settings.startup,
        ];
        let mut knob = [0.0; OPT_COUNT];
        for (k, v) in knob.iter_mut().zip(values) {
            *k = if v { 1.0 } else { 0.0 };
        }
        Self {
            visible: false,
            surface: None,
            knob,
            target: knob,
            hover: 0,
            tracking: false,
            dpi,
            view: 0,
            hotkey_capturing: false,
            reset_confirm: false,
            menu_open: false,
            lang_open: false,
            drag_slider: 0,
        }
    }

    fn hit(&self, x: i32, y: i32) -> i32 {
        // The language dropdown is modal: a click on a cell selects that
        // language, a click elsewhere only dismisses it.
        if self.lang_open {
            for i in 0..LANG_COUNT {
                let (cx, cy, cw, ch) = lang_cell_rect(i);
                if inside(x, y, cx, cy, cw, ch) {
                    return ID_LANG_BASE + i as i32;
                }
            }
            if inside(x, y, LANG_MENU_X, LANG_MENU_Y, LANG_MENU_W, LANG_MENU_H) {
                return ID_NONE;
            }
            if inside(x, y, LANG_BTN_X, LANG_BTN_Y, LANG_BTN_W, LANG_BTN_H) {
                return ID_LANG_SELECT;
            }
            if inside(x, y, BURGER_X, BURGER_Y, BURGER_W, BURGER_H) {
                return ID_MENU;
            }
            if inside(x, y, CLOSE_X, CLOSE_Y, CLOSE_W, CLOSE_H) {
                return ID_CLOSE;
            }
            return ID_NONE;
        }

        // The category dropdown is modal for the same reason.
        if self.menu_open {
            for i in 0..CATEGORY_COUNT {
                let ry = MENU_Y + MENU_PAD + MENU_HEADER_H + i as i32 * MENU_ROW_H;
                if inside(x, y, MENU_X, ry, MENU_W, MENU_ROW_H) {
                    return ID_CATEGORY_BASE + i as i32;
                }
            }
            if inside(x, y, MENU_X, MENU_Y, MENU_W, MENU_H) {
                return ID_NONE;
            }
            if inside(x, y, BURGER_X, BURGER_Y, BURGER_W, BURGER_H) {
                return ID_MENU;
            }
            if inside(x, y, CLOSE_X, CLOSE_Y, CLOSE_W, CLOSE_H) {
                return ID_CLOSE;
            }
            return ID_NONE;
        }

        if inside(x, y, CLOSE_X, CLOSE_Y, CLOSE_W, CLOSE_H) {
            return ID_CLOSE;
        }
        if inside(x, y, BURGER_X, BURGER_Y, BURGER_W, BURGER_H) {
            return ID_MENU;
        }
        if inside(x, y, 24, 444, PANEL_W - 48, 40) {
            return ID_QUIT;
        }
        match self.view {
            0 => {
                if inside(x, y, LANG_BTN_X, LANG_BTN_Y, LANG_BTN_W, LANG_BTN_H) {
                    return ID_LANG_SELECT;
                }
                for i in 0..OPT_COUNT {
                    let ry = OPT_ROW_Y + i as i32 * OPT_ROW_STEP + 5;
                    if inside(x, y, PANEL_W - 70, ry, 46, 26) {
                        return ID_OPT_BASE + i as i32;
                    }
                }
            }
            1 => {
                if self.reset_confirm {
                    if inside(x, y, 24, 370, 160, 34) { return ID_RESET_NO; }
                    if inside(x, y, 196, 370, 160, 34) { return ID_RESET_YES; }
                } else if inside(x, y, 24, 370, PANEL_W - 48, 34) {
                    return ID_RESET;
                }
            }
            2 => {
                if inside(x, y, 24, 224, PANEL_W - 48, 40) { return ID_HOTKEY; }
            }
            3 => {
                for (id, kind) in [
                    (ID_DURATION_SLIDER, SLIDER_DURATION),
                    (ID_PANEL_SLIDER, SLIDER_PANEL_OP),
                    (ID_OVERLAY_SLIDER, SLIDER_OVERLAY_OP),
                    (ID_SOUND_SLIDER, SLIDER_SOUND),
                ] {
                    let track_y = slider_track_y(kind);
                    if inside(
                        x, y,
                        SLIDER_X,
                        track_y + SLIDER_HIT_Y_OFF,
                        PANEL_W - 48,
                        SLIDER_HIT_H,
                    ) {
                        return id;
                    }
                }
                if inside(x, y, 24, 382, PANEL_W - 48, 34) { return ID_POSITION; }
            }
            _ => {}
        }
        ID_NONE
    }

    fn tick(&mut self) -> bool {
        let mut moving = false;
        for i in 0..OPT_COUNT {
            if (self.knob[i] - self.target[i]).abs() > 0.005 {
                self.knob[i] += (self.target[i] - self.knob[i]) * 0.35;
                moving = true;
            } else {
                self.knob[i] = self.target[i];
            }
        }
        moving
    }
}

struct PositionEditor {
    surface: Option<Surface>,       // full-work-area backdrop
    ghost_surface: Option<Surface>, // small live preview layer
    ghost_x: i32,
    ghost_y: i32,
    dragging: bool,
    off_x: i32,
    off_y: i32,
    dpi: u32,
}

impl PositionEditor {
    fn new(dpi: u32) -> Self {
        Self {
            surface: None,
            ghost_surface: None,
            ghost_x: 0,
            ghost_y: 0,
            dragging: false,
            off_x: 0,
            off_y: 0,
            dpi,
        }
    }

    fn begin(&mut self, settings: &Settings, dpi: u32) {
        let (w, h) = overlay_size(dpi);
        let (x, y) = overlay_xy(w, h, settings);
        self.ghost_x = x;
        self.ghost_y = y;
        self.dragging = false;
    }

    fn active(&self, w: i32, h: i32) -> usize {
        snap_index(self.ghost_x, self.ghost_y, w, h)
    }

    unsafe fn paint_backdrop(&mut self, hwnd: HWND, gfx: Option<&Gfx>, lang: Lang) {
        let (left, top, right, bottom) = work_area();
        let (ww, wh) = (right - left, bottom - top);
        if ww <= 0 || wh <= 0 {
            return;
        }
        if self.surface.as_ref().is_none_or(|s| s.w != ww || s.h != wh) {
            self.surface = Some(Surface::new(ww, wh));
        }
        let Some(surface) = self.surface.as_ref() else { return };
        if !surface.is_valid() {
            return;
        }
        let (w, h) = overlay_size(self.dpi);
        let anchors = snap_anchors(w, h);
        let active = snap_index(self.ghost_x, self.ghost_y, w, h);
        // One instruction card lives in the gap between slots 5 and 6.
        let top_card_bottom = anchors[4].1 - top + h;
        let center_top = anchors[5].1 - top;
        let banner_y = gap_center(top_card_bottom, center_top, 68);

        if let Some(gfx) = gfx {
            let Some(mut canvas) = gfx.canvas(surface.bits, ww, wh) else { return };
            canvas.set_language(lang);
            canvas.clear(0);
            // User scale: 100 = fully visible, 0 = invisible => 80% is alpha 204.
            // A zero-radius round_rect produces an empty GDI+ path; a 1px radius
            // fills the whole work area with the requested alpha.
            canvas.round_rect(
                0.0, 0.0, ww as f32, wh as f32, 1.0,
                (POS_DIM_ALPHA as u32) << 24, 0, 0.0,
            );

            // Instruction card at the top.
            let banner_w = 460.0f32.min(ww as f32 - 48.0);
            let banner_x = (ww as f32 - banner_w) / 2.0;
            canvas.round_rect(
                banner_x, banner_y as f32, banner_w, 68.0, 14.0,
                0xF017_1720, 0x33FF_FFFF, 1.0,
            );
            canvas.text(
                text(lang, "position_drag"), banner_x, (banner_y + 6) as f32, banner_w, 32.0,
                17.0, 0xFFFF_FFFF, true,
            );
            canvas.text(
                text(lang, "hotkey_cancel"), banner_x, (banner_y + 38) as f32, banner_w, 26.0,
                12.5, 0xFFC8_C8D0, true,
            );

            // Seven snap cards, drawn above the dim layer. The active anchor
            // is highlighted with the accent colour.
            for (i, (ax, ay)) in anchors.iter().enumerate() {
                let x = (ax - left) as f32;
                let y = (ay - top) as f32;
                let selected = i == active;
                let fill = if selected { 0x285E_E084 } else { 0x14FF_FFFF };
                let border = if selected { argb(C_ACCENT) } else { 0x59FF_FFFF };
                canvas.round_rect(
                    x, y, w as f32, h as f32, 10.0, fill, border,
                    if selected { 2.5 } else { 1.0 },
                );
                canvas.round_rect(
                    x + 8.0, y + 8.0, 22.0, 20.0, 7.0,
                    0xB014_141A, 0x44FF_FFFF, 1.0,
                );
                canvas.text(
                    &(i + 1).to_string(), x + 8.0, y + 8.0, 22.0, 20.0,
                    11.0, 0xFFE8_E8EE, true,
                );
            }

        } else {
            // GDI fallback: same visual language, without GDI+.
            for y in 0..wh {
                for x in 0..ww {
                    surface.put(x, y, [0, 0, 0, POS_DIM_ALPHA]);
                }
            }
            let draw_box = |rx: i32, ry: i32, bw: i32, bh: i32, r: f32, fill: [u8; 4], border: [u8; 4]| {
                for y in (ry - 1)..(ry + bh + 1) {
                    for x in (rx - 1)..(rx + bw + 1) {
                        let outer = rounded_coverage(x - rx, y - ry, bw, bh, r);
                        if outer == 0 {
                            continue;
                        }
                        let inner = rounded_coverage(
                            x - rx - 1, y - ry - 1, bw - 2, bh - 2, (r - 1.0).max(0.0),
                        );
                        let edge = outer.saturating_sub(inner);
                        let mut px = surface.get(x, y);
                        if fill[3] > 0 {
                            let a = (outer as u32 * fill[3] as u32 / 255) as u8;
                            blend(&mut px, [fill[0], fill[1], fill[2]], a);
                        }
                        if border[3] > 0 && edge > 0 {
                            let a = (edge as u32 * border[3] as u32 / 255) as u8;
                            blend(&mut px, [border[0], border[1], border[2]], a);
                        }
                        surface.put(x, y, px);
                    }
                }
            };
            let banner_w = 460.min(ww - 48);
            let banner_x = (ww - banner_w) / 2;
            draw_box(banner_x, banner_y, banner_w, 68, 14.0, [23, 23, 32, 240], [255, 255, 255, 50]);

            for (i, (ax, ay)) in anchors.iter().enumerate() {
                let (fill, border) = if i == active {
                    ([94, 224, 132, 40], [94, 224, 132, 255])
                } else {
                    ([0, 0, 0, 0], [255, 255, 255, 90])
                };
                draw_box(ax - left, ay - top, w, h, 10.0, fill, border);
            }

            let text_surface = Surface::new(ww, 30);
            for (y_at, key) in [
                (banner_y + 8, "position_drag"),
                (banner_y + 38, "hotkey_cancel"),
            ] {
                let mask = text_surface.text_mask(text(lang, key), 12.0, self.dpi, false);
                if mask.len() < (30 * ww) as usize {
                    continue;
                }
                for y in 0..30 {
                    for x in 0..ww {
                        let cov = mask[(y * ww + x) as usize];
                        if cov == 0 {
                            continue;
                        }
                        let mut px = surface.get(x, y_at + y);
                        blend(&mut px, [255, 255, 255], cov);
                        surface.put(x, y_at + y, px);
                    }
                }
            }
        }
        surface.premultiply();

        let screen = GetDC(core::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let old = SelectObject(mem, surface.bitmap);
        let pos = POINT { x: left, y: top };
        let size = SIZE { cx: ww, cy: wh };
        let src = POINT { x: 0, y: 0 };
        let blend_fn = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA,
        };
        UpdateLayeredWindow(hwnd, screen, &pos, &size, mem, &src, 0, &blend_fn, ULW_ALPHA);
        SelectObject(mem, old);
        DeleteDC(mem);
        ReleaseDC(core::ptr::null_mut(), screen);
    }

    /// Small preview layer. Its content never changes while dragging, so the
    /// smooth motion is only a SetWindowPos on this window per mouse move.
    unsafe fn paint_ghost(&mut self, hwnd: HWND, gfx: Option<&Gfx>, lang: Lang) {
        let (w, h) = overlay_size(self.dpi);
        if self.ghost_surface.as_ref().is_none_or(|s| s.w != w || s.h != h) {
            self.ghost_surface = Some(Surface::new(w, h));
        }
        let Some(surface) = self.ghost_surface.as_ref() else { return };
        if !surface.is_valid() {
            return;
        }

        if let Some(gfx) = gfx {
            let Some(mut canvas) = gfx.canvas(surface.bits, w, h) else { return };
            canvas.set_language(lang);
            canvas.clear(0);
            canvas.round_rect(
                4.0, 6.0, w as f32, h as f32, 10.0, 0x6600_0000, 0, 0.0,
            );
            gfx.draw_indicator(&mut canvas, true, 0.0, 0.0, w as f32, h as f32);
            canvas.round_rect(
                0.0, 0.0, w as f32, h as f32, 10.0, 0, argb(C_ACCENT), 2.5,
            );
            canvas.text(
                text(lang, "caps_on"), 0.0, 0.0, w as f32, h as f32,
                15.0, 0xFFFF_FFFF, true,
            );
        } else {
            surface.clear();
            for y in 0..h {
                for x in 0..w {
                    let cov = rounded_coverage(
                        x, y, w, h,
                        (10.0f32).min(w as f32 / 2.0).min(h as f32 / 2.0),
                    );
                    if cov == 0 {
                        continue;
                    }
                    let mut px = surface.get(x, y);
                    blend(&mut px, [20, 20, 20], cov);
                    surface.put(x, y, px);
                }
            }
            let label = Surface::new(w, h);
            let mask = label.text_mask(text(lang, "caps_on"), 15.0, self.dpi, true);
            if mask.len() < (w * h) as usize {
                return;
            }
            for y in 0..h {
                for x in 0..w {
                    let cov = mask[(y * w + x) as usize];
                    if cov == 0 {
                        continue;
                    }
                    let mut px = surface.get(x, y);
                    blend(&mut px, [255, 255, 255], cov);
                    surface.put(x, y, px);
                }
            }
        }
        surface.premultiply();

        let screen = GetDC(core::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let old = SelectObject(mem, surface.bitmap);
        let pos = POINT { x: self.ghost_x, y: self.ghost_y };
        let size = SIZE { cx: w, cy: h };
        let src = POINT { x: 0, y: 0 };
        let blend_fn = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA,
        };
        UpdateLayeredWindow(hwnd, screen, &pos, &size, mem, &src, 0, &blend_fn, ULW_ALPHA);
        SelectObject(mem, old);
        DeleteDC(mem);
        ReleaseDC(core::ptr::null_mut(), screen);
    }

    fn mouse_down(&mut self, x: i32, y: i32, left: i32, top: i32, w: i32, h: i32) {
        let sx = x + left;
        let sy = y + top;
        if inside(sx, sy, self.ghost_x, self.ghost_y, w, h) {
            self.off_x = sx - self.ghost_x;
            self.off_y = sy - self.ghost_y;
        } else {
            self.off_x = w / 2;
            self.off_y = h / 2;
            self.ghost_x = sx - self.off_x;
            self.ghost_y = sy - self.off_y;
        }
        self.dragging = true;
    }

    fn mouse_move(&mut self, x: i32, y: i32, left: i32, top: i32, w: i32, h: i32) {
        if !self.dragging { return }
        let (_, _, right, bottom) = work_area();
        self.ghost_x = (x + left - self.off_x).clamp(left, right - w);
        self.ghost_y = (y + top - self.off_y).clamp(top, bottom - h);
        let anchors = snap_anchors(w, h);
        let (ax, ay) = anchors[snap_index(self.ghost_x, self.ghost_y, w, h)];
        let dx = (ax - self.ghost_x) as i64;
        let dy = (ay - self.ghost_y) as i64;
        if dx * dx + dy * dy <= 90 * 90 {
            self.ghost_x = ax;
            self.ghost_y = ay;
        }
    }

    fn mouse_up(&mut self) -> (i32, i32) {
        self.dragging = false;
        (self.ghost_x, self.ghost_y)
    }
}

struct App {
    settings: Settings,
    /// Last startup flag written to the registry, so unrelated setting changes
    /// do not needlessly touch HKCU every time a slider moves.
    startup_applied: bool,
    // Pre-scaled WAVs stay alive for the whole async playback duration.
    sound_on: Vec<Vec<u8>>,
    sound_off: Vec<Vec<u8>>,
    gfx: Option<Gfx>,
    hinstance: HINSTANCE,
    hwnd_msg: HWND,
    hwnd_overlay: HWND,
    hwnd_panel: HWND,
    hwnd_pos: HWND,
    hwnd_ghost: HWND,
    hicon: HICON,
    hook: HHOOK,
    caps_on: bool,
    num_on: bool,
    key_down: [bool; 2], // 0 = Caps Lock, 1 = Num Lock
    mod_down: u8,        // live modifier mask from the hook: 1=Alt 2=Ctrl 4=Shift 8=Win
    hotkey_down: bool,
    caps_started: Option<Instant>,
    num_started: Option<Instant>,
    overlay: Overlay,
    panel: Panel,
    pos: PositionEditor,
    timer_overlay: bool,
    timer_panel: bool,
    pos_open: bool,
    log: Option<File>,
    quitting: bool,
}

impl App {
    fn new(hinstance: HINSTANCE) -> Self {
        let dpi = unsafe { GetDpiForSystem() };
        let settings = Settings::load();
        let sound_on = SOUND_STEPS.iter().map(|&v| scaled_wav(SND_ON, v)).collect();
        let sound_off = SOUND_STEPS.iter().map(|&v| scaled_wav(SND_OFF, v)).collect();
        let gfx = unsafe { Gfx::new() };
        let startup_applied = settings.startup;
        let panel = Panel::new(&settings, dpi);
        let log = if std::env::args().any(|a| a == "--debug") {
            Settings::log_path().and_then(|path| {
                create_dir_all(path.parent()?).ok()?;
                OpenOptions::new().create(true).append(true).open(path).ok()
            })
        } else {
            None
        };
        Self {
            settings,
            startup_applied,
            sound_on,
            sound_off,
            gfx,
            hinstance,
            hwnd_msg: core::ptr::null_mut(),
            hwnd_overlay: core::ptr::null_mut(),
            hwnd_panel: core::ptr::null_mut(),
            hwnd_pos: core::ptr::null_mut(),
            hwnd_ghost: core::ptr::null_mut(),
            hicon: core::ptr::null_mut(),
            hook: core::ptr::null_mut(),
            caps_on: false,
            num_on: false,
            key_down: [false; 2],
            mod_down: unsafe { modifier_mask() },
            hotkey_down: false,
            caps_started: None,
            num_started: None,
            overlay: Overlay::new(dpi),
            panel,
            pos: PositionEditor::new(dpi),
            timer_overlay: false,
            timer_panel: false,
            pos_open: false,
            log,
            quitting: false,
        }
    }

    fn log(&mut self, message: &str) {
        let Some(file) = self.log.as_mut() else { return };
        let mut t = SYSTEMTIME::default();
        unsafe { GetLocalTime(&mut t) };
        let _ = writeln!(
            file,
            "{:02}:{:02}:{:02}.{:03} {message}",
            t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
        );
        let _ = file.flush();
    }

    unsafe fn init(&mut self) -> bool {
        let class = wide(CLASS_NAME);
        let wc = WNDCLASSEXW {
            cbSize: core::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wnd_proc),
            cbWndExtra: 8, // one pointer: window kind
            hInstance: self.hinstance,
            lpszClassName: class.as_ptr(),
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            return false;
        }

        let ci = CreateInfo { app: self as *mut App, kind: KIND_MSG };
        self.hwnd_msg = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("IndiCaps").as_ptr(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            core::ptr::null_mut(),
            self.hinstance,
            &ci as *const _ as *const core::ffi::c_void,
        );

        let ci = CreateInfo { app: self as *mut App, kind: KIND_OVERLAY };
        self.hwnd_overlay = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
            class.as_ptr(),
            wide("IndiCaps overlay").as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            self.hinstance,
            &ci as *const _ as *const core::ffi::c_void,
        );
        if self.hwnd_msg.is_null() || self.hwnd_overlay.is_null() {
            // Without the message window the tray and low-level hook cannot
            // work; fail early instead of running invisible and mute.
            return false;
        }

        self.hicon = match self.gfx.as_ref() {
            Some(gfx) => unsafe { gfx.tray_icon() },
            None => create_tray_icon(),
        };
        if self.hicon.is_null() {
            self.hicon = create_tray_icon();
        }
        self.add_tray();
        self.caps_on = (GetKeyState(VK_CAPITAL) & 1) != 0;
        self.num_on = (GetKeyState(VK_NUMLOCK) & 1) != 0;
        if self.caps_on { self.caps_started = Some(Instant::now()); }
        if self.num_on { self.num_started = Some(Instant::now()); }
        SetTimer(self.hwnd_msg, TIMER_STATS, 30_000, core::ptr::null());
        self.hook = SetWindowsHookExW(WH_KEYBOARD_LL, hook_proc, self.hinstance, 0);
        let state = GetKeyState(VK_CAPITAL) & 1;
        self.log(&format!("start capsOn={} keystate={state}", self.caps_on));
        true
    }

    unsafe fn run(&mut self) {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, core::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    unsafe fn shutdown(&mut self) {
        if !self.hook.is_null() {
            UnhookWindowsHookEx(self.hook);
            self.hook = core::ptr::null_mut();
        }
        KillTimer(self.hwnd_msg, TIMER_STATS);
        self.flush_stats();
        if !self.hwnd_pos.is_null() {
            DestroyWindow(self.hwnd_pos);
            self.hwnd_pos = core::ptr::null_mut();
        }
        if !self.hwnd_ghost.is_null() {
            DestroyWindow(self.hwnd_ghost);
            self.hwnd_ghost = core::ptr::null_mut();
        }
        self.pos_open = false;
        self.remove_tray();
        if !self.hicon.is_null() {
            DestroyIcon(self.hicon);
            self.hicon = core::ptr::null_mut();
        }
        for hwnd in [self.hwnd_panel, self.hwnd_overlay, self.hwnd_msg] {
            if !hwnd.is_null() {
                DestroyWindow(hwnd);
            }
        }
        self.hwnd_panel = core::ptr::null_mut();
        self.hwnd_overlay = core::ptr::null_mut();
        self.hwnd_msg = core::ptr::null_mut();
    }

    fn add_tray(&self) {
        unsafe {
            let mut data = NOTIFYICONDATAW {
                cbSize: core::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd_msg,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: WM_APP_TRAY,
                hIcon: self.hicon,
                ..Default::default()
            };
            for (dst, src) in data.szTip.iter_mut().zip(wide("IndiCaps 2.0.0 · by Jovatis")) {
                *dst = src;
            }
            Shell_NotifyIconW(NIM_ADD, &data);
        }
    }

    fn remove_tray(&self) {
        if self.hwnd_msg.is_null() {
            return;
        }
        unsafe {
            let data = NOTIFYICONDATAW {
                cbSize: core::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd_msg,
                uID: 1,
                ..Default::default()
            };
            Shell_NotifyIconW(NIM_DELETE, &data);
        }
    }

    /// Adds the currently running "on" periods to the persisted counters, then
    /// clears the timers. The caller restarts them according to the new state.
    fn accrue_stats(&mut self) {
        if let Some(started) = self.caps_started.take() {
            self.settings.caps_on_ms = self
                .settings
                .caps_on_ms
                .saturating_add(started.elapsed().as_millis() as u64);
        }
        if let Some(started) = self.num_started.take() {
            self.settings.num_on_ms = self
                .settings
                .num_on_ms
                .saturating_add(started.elapsed().as_millis() as u64);
        }
    }

    fn flush_stats(&mut self) {
        self.accrue_stats();
        let now = Instant::now();
        if self.caps_on {
            self.caps_started = Some(now);
        }
        if self.num_on {
            self.num_started = Some(now);
        }
        self.settings.save();
    }

    fn reset_stats(&mut self) {
        let now = Instant::now();
        self.settings.caps_toggles = 0;
        self.settings.num_toggles = 0;
        self.settings.caps_on_ms = 0;
        self.settings.num_on_ms = 0;
        self.settings.stats_started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        self.caps_started = if self.caps_on { Some(now) } else { None };
        self.num_started = if self.num_on { Some(now) } else { None };
        self.settings.save();
    }

    fn on_caps_change(&mut self) {
        // Always trust the system toggle bit instead of toggling our own copy.
        let state = unsafe { GetKeyState(VK_CAPITAL) & 1 } != 0;
        self.log(&format!("caps state={} cached={}", state, self.caps_on));
        if state == self.caps_on {
            return; // key-up echo or delayed event: nothing actually changed
        }
        self.accrue_stats();
        self.caps_on = state;
        self.caps_started = if state { Some(Instant::now()) } else { None };
        self.settings.caps_toggles = self.settings.caps_toggles.saturating_add(1);
        self.announce_state(state, self.settings.indicator, "caps_on", "caps_off");
        self.settings.save();
        self.invalidate_panel();
    }

    fn on_num_change(&mut self) {
        let state = unsafe { GetKeyState(VK_NUMLOCK) & 1 } != 0;
        self.log(&format!("num state={} cached={}", state, self.num_on));
        if state == self.num_on {
            return;
        }
        self.accrue_stats();
        self.num_on = state;
        self.num_started = if state { Some(Instant::now()) } else { None };
        self.settings.num_toggles = self.settings.num_toggles.saturating_add(1);
        self.announce_state(state, self.settings.num_indicator, "num_on", "num_off");
        self.settings.save();
        self.invalidate_panel();
    }

    fn announce_state(&mut self, state: bool, enabled: bool, key_on: &'static str, key_off: &'static str) {
        // A sound is heard only when sounds are enabled AND this indicator is
        // enabled. Disabling Num Lock in the panel also silences its sound.
        self.log(&format!("sound gate: global={} indicator={}", self.settings.sound, enabled));
        if self.settings.sound && enabled && self.settings.sound_volume > 0 {
            let bank = (self.settings.sound_volume / 10) as usize;
            let bank = bank.min(SOUND_STEPS.len() - 1);
            let sound = if state { &self.sound_on[bank] } else { &self.sound_off[bank] };
            unsafe { PlaySoundW(sound.as_ptr() as *const u16, core::ptr::null_mut(), SND_ASYNC | SND_MEMORY) };
            self.log("sound played");
        }
        if enabled {
            let key = if state { key_on } else { key_off };
            let caption = text(self.settings.language, key).to_string();
            let animate = self.settings.animation;
            self.overlay.display(
                self.hwnd_overlay,
                &caption,
                key,
                state,
                animate,
                self.gfx.as_ref(),
                &self.settings,
            );
            if !self.timer_overlay {
                unsafe { SetTimer(self.hwnd_msg, TIMER_OVERLAY, 12, core::ptr::null()) };
                self.timer_overlay = true;
            }
        }
    }

    fn start_panel_timer(&mut self) {
        if !self.timer_panel {
            unsafe { SetTimer(self.hwnd_msg, TIMER_PANEL, 12, core::ptr::null()) };
            self.timer_panel = true;
        }
    }

    fn on_timer(&mut self, id: usize) {
        if id == TIMER_OVERLAY {
            let alive = self.overlay.tick(self.hwnd_overlay);
            if !alive {
                unsafe { KillTimer(self.hwnd_msg, TIMER_OVERLAY) };
                self.timer_overlay = false;
            }
        } else if id == TIMER_PANEL {
            let moving = self.panel.tick();
            if moving {
                self.invalidate_panel();
            }
            let alive = moving;
            if !alive {
                unsafe { KillTimer(self.hwnd_msg, TIMER_PANEL) };
                self.timer_panel = false;
            }
        } else if id == TIMER_STATS {
            self.flush_stats();
            self.invalidate_panel();
        }
    }

    fn show_tray_menu(&mut self) {
        unsafe {
            let lang = self.settings.language;
            let open = wide(text(lang, "menu_open"));
            let quit = wide(text(lang, "menu_quit"));
            let menu = CreatePopupMenu();
            AppendMenuW(menu, MF_STRING, 1, open.as_ptr());
            AppendMenuW(menu, MF_STRING, 2, quit.as_ptr());
            let mut pos = POINT::default();
            GetCursorPos(&mut pos);
            SetForegroundWindow(self.hwnd_msg);
            let cmd = TrackPopupMenuEx(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                pos.x,
                pos.y,
                self.hwnd_msg,
                core::ptr::null(),
            );
            DestroyMenu(menu);
            PostMessageW(self.hwnd_msg, WM_NULL, 0, 0);
            match cmd {
                1 => self.toggle_panel(),
                2 => self.quit(),
                _ => {}
            }
        }
    }

    fn quit(&mut self) {
        if self.quitting {
            return;
        }
        self.quitting = true;
        if self.pos_open {
            unsafe { ShowWindow(self.hwnd_pos, SW_HIDE) };
            self.pos_open = false;
            self.pos.dragging = false;
            self.pos.surface = None;
        }
        self.remove_tray();
        unsafe { PostQuitMessage(0) };
    }

    unsafe fn create_panel(&mut self) {
        let class = wide(CLASS_NAME);
        let ci = CreateInfo { app: self as *mut App, kind: KIND_PANEL };
        let layered = self.gfx.is_some();
        let ex_style = if layered { WS_EX_LAYERED | WS_EX_TOOLWINDOW } else { WS_EX_TOOLWINDOW };
        let hwnd = CreateWindowExW(
            ex_style,
            class.as_ptr(),
            wide("IndiCaps settings").as_ptr(),
            WS_POPUP,
            0,
            0,
            PANEL_W,
            PANEL_H,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            self.hinstance,
            &ci as *const _ as *const core::ffi::c_void,
        );
        if !layered {
            // Only the GDI fallback needs a hard-clipped region; the layered
            // panel gets smooth per-pixel alpha corners from UpdateLayeredWindow.
            let region = CreateRoundRectRgn(0, 0, PANEL_W, PANEL_H, 18, 18);
            if SetWindowRgn(hwnd, region, 1) == 0 {
                // The window takes ownership only on success.
                DeleteObject(region);
            }
        }
        self.hwnd_panel = hwnd;
    }

    fn hide_panel(&mut self) {
        if self.hwnd_panel.is_null() || !self.panel.visible {
            return;
        }
        unsafe { ShowWindow(self.hwnd_panel, SW_HIDE) };
        self.panel.visible = false;
        self.panel.menu_open = false;
        self.panel.lang_open = false;
        self.panel.hotkey_capturing = false;
        self.panel.reset_confirm = false;
        self.panel.drag_slider = 0;
    }

    fn toggle_panel(&mut self) {
        if self.panel.visible {
            self.hide_panel();
            return;
        }
        if self.hwnd_panel.is_null() {
            unsafe { self.create_panel() };
        }
        let (x, y) = panel_pos();
        unsafe {
            SetWindowPos(
                self.hwnd_panel,
                HWND_TOPMOST,
                x,
                y,
                PANEL_W,
                PANEL_H,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            ShowWindow(self.hwnd_panel, SW_SHOW);
            SetForegroundWindow(self.hwnd_panel);
        }
        self.panel.visible = true;
        self.panel.hover = 0;
        self.panel.menu_open = false;
        self.panel.drag_slider = 0;
        self.invalidate_panel();
    }

    unsafe fn create_pos(&mut self) {
        let class = wide(CLASS_NAME);
        let ci = CreateInfo { app: self as *mut App, kind: KIND_POS };
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class.as_ptr(),
            wide("IndiCaps position").as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            self.hinstance,
            &ci as *const _ as *const core::ffi::c_void,
        );
        self.hwnd_pos = hwnd;
    }

    unsafe fn create_ghost(&mut self) {
        let class = wide(CLASS_NAME);
        let ci = CreateInfo { app: self as *mut App, kind: KIND_GHOST };
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST |
                WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
            class.as_ptr(),
            wide("IndiCaps position preview").as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            self.hinstance,
            &ci as *const _ as *const core::ffi::c_void,
        );
        self.hwnd_ghost = hwnd;
    }

    fn position_ghost_window(&mut self) {
        if self.hwnd_ghost.is_null() || !self.pos_open {
            return;
        }
        let (w, h) = overlay_size(self.pos.dpi);
        unsafe {
            SetWindowPos(
                self.hwnd_ghost,
                HWND_TOPMOST,
                self.pos.ghost_x,
                self.pos.ghost_y,
                w,
                h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    fn open_position_editor(&mut self) {
        self.hide_panel();
        if self.hwnd_pos.is_null() {
            unsafe { self.create_pos() };
        }
        if self.hwnd_ghost.is_null() {
            unsafe { self.create_ghost() };
        }
        self.pos.begin(&self.settings, self.pos.dpi);
        let (left, top, right, bottom) = work_area();
        unsafe {
            SetWindowPos(
                self.hwnd_pos,
                HWND_TOPMOST,
                left,
                top,
                right - left,
                bottom - top,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            ShowWindow(self.hwnd_pos, SW_SHOW);
            SetForegroundWindow(self.hwnd_pos);
        }
        self.pos_open = true;
        self.position_ghost_window();
        unsafe { self.invalidate_pos() };
    }

    fn close_position_editor(&mut self) {
        if self.hwnd_pos.is_null() {
            return;
        }
        unsafe { ShowWindow(self.hwnd_pos, SW_HIDE) };
        if !self.hwnd_ghost.is_null() {
            unsafe { ShowWindow(self.hwnd_ghost, SW_HIDE) };
        }
        self.pos_open = false;
        self.pos.dragging = false;
        self.pos.surface = None; // release the full-work-area DIB immediately
        self.toggle_panel();
    }

    unsafe fn invalidate_pos(&mut self) {
        if self.hwnd_pos.is_null() || !self.pos_open {
            return;
        }
        self.pos.paint_backdrop(self.hwnd_pos, self.gfx.as_ref(), self.settings.language);
        if !self.hwnd_ghost.is_null() {
            self.pos.paint_ghost(self.hwnd_ghost, self.gfx.as_ref(), self.settings.language);
        }
    }

    fn apply_settings(&mut self) {
        self.settings.save();
        if self.settings.startup != self.startup_applied {
            self.settings.apply_startup();
            self.startup_applied = self.settings.startup;
        }
    }

    fn panel_click(&mut self, x: i32, y: i32) {
        let id = self.panel.hit(x, y);

        // Language dropdown is modal: select a language or just dismiss it.
        if self.panel.lang_open {
            self.panel.lang_open = false;
            match id {
                id if (ID_LANG_BASE..ID_LANG_BASE + LANG_COUNT as i32).contains(&id) => {
                    let lang = LANGS[(id - ID_LANG_BASE) as usize];
                    self.settings.language = lang;
                    self.overlay.set_language(lang, self.gfx.as_ref());
                    self.apply_settings();
                    self.log(&format!("panel language {}", lang.code()));
                }
                ID_MENU => {
                    self.panel.menu_open = true;
                }
                ID_CLOSE => {
                    self.invalidate_panel();
                    self.hide_panel();
                    return;
                }
                _ => {}
            }
            self.invalidate_panel();
            return;
        }

        // The burger dropdown is modal: a first click outside simply dismisses it.
        if self.panel.menu_open {
            self.panel.menu_open = false;
            match id {
                id if (ID_CATEGORY_BASE..ID_CATEGORY_BASE + CATEGORY_COUNT as i32)
                    .contains(&id) =>
                {
                    self.panel.view = (id - ID_CATEGORY_BASE) as u8;
                    self.panel.hotkey_capturing = false;
                    self.panel.reset_confirm = false;
                    self.log(&format!("panel category {}", self.panel.view));
                }
                ID_CLOSE => {
                    self.invalidate_panel();
                    self.hide_panel();
                    return;
                }
                _ => {}
            }
            self.invalidate_panel();
            return;
        }

        match id {
            ID_CLOSE => self.hide_panel(),
            ID_MENU => {
                self.panel.menu_open = true;
                self.panel.lang_open = false;
                self.invalidate_panel();
            }
            ID_LANG_SELECT => {
                self.panel.lang_open = true;
                self.panel.menu_open = false;
                self.invalidate_panel();
            }
            ID_QUIT => self.quit(),
            id if (ID_OPT_BASE..ID_OPT_BASE + OPT_COUNT as i32).contains(&id) => {
                let i = (id - ID_OPT_BASE) as usize;
                let value = match i {
                    0 => !self.settings.indicator,
                    1 => !self.settings.num_indicator,
                    2 => !self.settings.sound,
                    3 => !self.settings.animation,
                    _ => !self.settings.startup,
                };
                match i {
                    0 => self.settings.indicator = value,
                    1 => self.settings.num_indicator = value,
                    2 => self.settings.sound = value,
                    3 => self.settings.animation = value,
                    _ => self.settings.startup = value,
                }
                self.panel.target[i] = if value { 1.0 } else { 0.0 };
                self.apply_settings();
                self.start_panel_timer();
                self.invalidate_panel();
            }

            ID_RESET => {
                self.panel.reset_confirm = true;
                self.invalidate_panel();
            }
            ID_RESET_NO => {
                self.panel.reset_confirm = false;
                self.invalidate_panel();
            }
            ID_RESET_YES => {
                self.panel.reset_confirm = false;
                self.reset_stats();
                self.invalidate_panel();
            }
            ID_HOTKEY => {
                self.panel.hotkey_capturing = true;
                self.log("hotkey capture requested");
                self.invalidate_panel();
            }
            id if slider_kind(id) != 0 => {
                let kind = slider_kind(id);
                let idx = slider_index_from_x(x, kind);
                slider_apply(&mut self.settings, kind, idx);
                // Persist once on mouse-up instead of on every click/drag step.
                self.invalidate_panel();
            }
            ID_POSITION => self.open_position_editor(),
            _ => {}
        }
    }

    fn panel_slider_drag(&mut self, x: i32) {
        let kind = self.panel.drag_slider;
        if kind == 0 {
            return;
        }
        let idx = slider_index_from_x(x, kind);
        if slider_values_differ(&self.settings, kind, idx) {
            slider_apply(&mut self.settings, kind, idx);
            self.invalidate_panel();
        }
    }

    fn invalidate_panel(&mut self) {
        if self.hwnd_panel.is_null() || !self.panel.visible {
            return;
        }
        if self.gfx.is_some() {
            unsafe { self.paint_panel_layered() };
        } else {
            unsafe { InvalidateRect(self.hwnd_panel, core::ptr::null(), 1) };
        }
    }

    /// Renders the whole panel as a layered window: every curve and glyph is
    /// anti-aliased by GDI+, including the outer rounded corners.
    unsafe fn paint_panel_layered(&mut self) {
        if self.panel.surface.as_ref().is_none_or(|s| s.w != PANEL_W || s.h != PANEL_H) {
            self.panel.surface = Some(Surface::new(PANEL_W, PANEL_H));
        }
        let Some(gfx) = self.gfx.as_ref() else { return };
        let Some(surface) = self.panel.surface.as_ref() else { return };
        if !surface.is_valid() {
            return;
        }
        let lang = self.settings.language;
        let view = self.panel.view;
        let hover = self.panel.hover;

        {
            let Some(mut canvas) = gfx.canvas(surface.bits, PANEL_W, PANEL_H) else { return };
            canvas.set_language(lang);
            canvas.clear(0);
            canvas.round_rect(
                0.5, 0.5, PANEL_W as f32 - 1.0, PANEL_H as f32 - 1.0,
                17.5, argb(C_BG), argb(C_BORDER), 1.0,
            );

            // Header: logo + title on one line, subtitle under the group.
            canvas.image(
                gfx.logo(), HEADER_LOGO_X as f32, HEADER_LOGO_Y as f32,
                HEADER_LOGO_SIZE as f32, HEADER_LOGO_SIZE as f32,
            );
            canvas.text(
                "IndiCaps", HEADER_TITLE_X as f32, HEADER_TITLE_Y as f32,
                HEADER_TITLE_W as f32, HEADER_TITLE_H as f32,
                24.0, argb(C_TEXT), false,
            );
            canvas.text(
                text(lang, CATEGORY_KEYS[view as usize]),
                HEADER_SUBTITLE_X as f32, HEADER_SUBTITLE_Y as f32,
                HEADER_SUBTITLE_W as f32, HEADER_SUBTITLE_H as f32,
                15.0, argb(C_SECONDARY), false,
            );

            // Burger button stays three bars even while its dropdown is open;
            // clicking it again closes the dropdown.
            let burger_active = self.panel.menu_open || hover == ID_MENU;
            canvas.round_rect(
                BURGER_X as f32, BURGER_Y as f32, BURGER_W as f32, BURGER_H as f32, 8.0,
                if burger_active { argb(C_SELECT) } else { argb(C_TRACK) },
                if burger_active { argb(C_ACCENT) } else { argb(C_BORDER) },
                1.0,
            );
            let icon = if burger_active { argb(C_ACCENT) } else { argb(C_TEXT) };
            for (i, &len) in [16.0f32, 12.0, 16.0].iter().enumerate() {
                let lx = BURGER_X as f32 + (BURGER_W as f32 - len) / 2.0;
                let ly = BURGER_Y as f32 + 8.0 + i as f32 * 6.0;
                canvas.round_rect(lx, ly, len, 2.0, 1.0, icon, 0, 0.0);
            }

            canvas.round_rect(
                CLOSE_X as f32, CLOSE_Y as f32, CLOSE_W as f32, CLOSE_H as f32, 8.0,
                if hover == ID_CLOSE { argb(C_HOVER) } else { argb(C_TRACK) },
                argb(C_BORDER), 1.0,
            );
            canvas.line(
                (CLOSE_X + 9) as f32, (CLOSE_Y + 9) as f32,
                (CLOSE_X + CLOSE_W - 9) as f32, (CLOSE_Y + CLOSE_H - 9) as f32,
                argb(C_TEXT), 1.6,
            );
            canvas.line(
                (CLOSE_X + CLOSE_W - 9) as f32, (CLOSE_Y + 9) as f32,
                (CLOSE_X + 9) as f32, (CLOSE_Y + CLOSE_H - 9) as f32,
                argb(C_TEXT), 1.6,
            );

            canvas.line(24.0, 84.5, (PANEL_W - 24) as f32, 84.5, argb(C_BORDER), 1.0);
            canvas.line(24.0, 84.5, 96.0, 84.5, argb(C_ACCENT), 2.0);

            match view {
                0 => {
                    canvas.text(
                        text(lang, "lang_label"), 24.0, LANG_LABEL_Y as f32, 200.0, 22.0,
                        13.0, argb(C_SECONDARY), false,
                    );

                    // Single selector showing the active language in its own script.
                    let lang_open = self.panel.lang_open;
                    let (btn_fill, btn_border) = if lang_open {
                        (argb(C_SELECT), argb(C_ACCENT))
                    } else if hover == ID_LANG_SELECT {
                        (argb(C_HOVER), argb(C_BORDER))
                    } else {
                        (argb(C_TRACK), argb(C_BORDER))
                    };
                    canvas.round_rect(
                        LANG_BTN_X as f32, LANG_BTN_Y as f32, LANG_BTN_W as f32,
                        LANG_BTN_H as f32, 10.0, btn_fill, btn_border, 1.0,
                    );
                    canvas.set_language(lang);
                    canvas.text(
                        lang.native(), (LANG_BTN_X + 14) as f32, LANG_BTN_Y as f32,
                        (LANG_BTN_W - 52) as f32, LANG_BTN_H as f32,
                        15.0, argb(C_TEXT), false,
                    );
                    let chevron_x = (LANG_BTN_X + LANG_BTN_W - 26) as f32;
                    let chevron_y = (LANG_BTN_Y + LANG_BTN_H / 2) as f32;
                    if lang_open {
                        canvas.line(
                            chevron_x - 5.0, chevron_y + 2.0,
                            chevron_x, chevron_y - 3.0, argb(C_ACCENT), 1.8,
                        );
                        canvas.line(
                            chevron_x, chevron_y - 3.0,
                            chevron_x + 5.0, chevron_y + 2.0, argb(C_ACCENT), 1.8,
                        );
                    } else {
                        canvas.line(
                            chevron_x - 5.0, chevron_y - 2.0,
                            chevron_x, chevron_y + 3.0, argb(C_SECONDARY), 1.8,
                        );
                        canvas.line(
                            chevron_x, chevron_y + 3.0,
                            chevron_x + 5.0, chevron_y - 2.0, argb(C_SECONDARY), 1.8,
                        );
                    }

                    canvas.line(24.0, 166.5, (PANEL_W - 24) as f32, 166.5, argb(C_BORDER), 1.0);

                    let values = [
                        self.settings.indicator,
                        self.settings.num_indicator,
                        self.settings.sound,
                        self.settings.animation,
                        self.settings.startup,
                    ];
                    for i in 0..OPT_COUNT {
                        let y = (OPT_ROW_Y + i as i32 * OPT_ROW_STEP) as f32;
                        canvas.text(
                            text(lang, OPT_KEYS[i]), 24.0, y, 270.0, OPT_ROW_H as f32,
                            16.0, argb(C_TEXT), false,
                        );
                        let sx = (PANEL_W - 70) as f32;
                        let sy = y + 5.0;
                        let track = if values[i] { argb(C_ACCENT) } else { argb(C_TRACK) };
                        canvas.round_rect(
                            sx, sy, 46.0, 26.0, 13.0, track, argb(C_BORDER), 1.0,
                        );
                        let knob = self.panel.knob[i].clamp(0.0, 1.0);
                        let kx = sx + 3.0 + knob * 20.0;
                        canvas.ellipse(kx - 1.0, sy + 2.0, 22.0, 22.0, 0x3300_0000, 0, 0.0);
                        canvas.ellipse(kx, sy + 3.0, 20.0, 20.0, argb(C_TEXT), 0, 0.0);
                    }
                }
                1 => {
                    canvas.text(text(lang, "stats_title"), 24.0, 110.0, 332.0, 30.0, 17.0, argb(C_TEXT), false);
                    let rows = [
                        ("stat_caps", self.settings.caps_toggles.to_string()),
                        ("stat_num", self.settings.num_toggles.to_string()),
                        ("stat_caps_time", format_duration(self.settings.caps_on_ms)),
                        ("stat_num_time", format_duration(self.settings.num_on_ms)),
                        ("stat_since", stats_since(&self.settings)),
                    ];
                    for (i, (key, value)) in rows.iter().enumerate() {
                        let y = (146 + i as i32 * 38) as f32;
                        canvas.text(text(lang, key), 24.0, y, 180.0, 28.0, 14.0, argb(C_SECONDARY), false);
                        canvas.text(value, 204.0, y, 152.0, 28.0, 15.0, argb(C_TEXT), true);
                    }
                    if self.panel.reset_confirm {
                        canvas.text(
                            text(lang, "stat_reset_confirm"), 24.0, 334.0, 332.0, 26.0,
                            12.5, argb(C_ACCENT), true,
                        );
                        let no_fill = if hover == ID_RESET_NO { argb(C_HOVER) } else { argb(C_TRACK) };
                        canvas.round_rect(24.0, 370.0, 160.0, 34.0, 9.0, no_fill, argb(C_BORDER), 1.0);
                        canvas.text(text(lang, "stat_reset_no"), 24.0, 370.0, 160.0, 34.0, 14.0, argb(C_TEXT), true);
                        let yes_fill = argb(C_ACCENT);
                        canvas.round_rect(196.0, 370.0, 160.0, 34.0, 9.0, yes_fill, yes_fill, 1.0);
                        canvas.text(text(lang, "stat_reset_yes"), 196.0, 370.0, 160.0, 34.0, 14.0, argb(C_BG), true);
                    } else {
                        canvas.text(text(lang, "stat_local"), 24.0, 334.0, 332.0, 26.0, 12.5, argb(C_SECONDARY), true);
                        let fill = if hover == ID_RESET { argb(C_HOVER) } else { argb(C_TRACK) };
                        canvas.round_rect(24.0, 370.0, (PANEL_W - 48) as f32, 34.0, 9.0, fill, argb(C_BORDER), 1.0);
                        canvas.text(text(lang, "stat_reset"), 24.0, 370.0, (PANEL_W - 48) as f32, 34.0, 14.0, argb(C_TEXT), true);
                    }
                }
                2 => {
                    canvas.text(text(lang, "hotkey_title"), 24.0, 110.0, 332.0, 30.0, 17.0, argb(C_TEXT), false);
                    if self.panel.hotkey_capturing {
                        canvas.text(text(lang, "hotkey_wait"), 24.0, 156.0, 332.0, 34.0, 18.0, argb(C_ACCENT), true);
                        canvas.text(text(lang, "hotkey_cancel"), 24.0, 194.0, 332.0, 26.0, 13.0, argb(C_SECONDARY), true);
                    } else {
                        canvas.text(text(lang, "hotkey_current"), 24.0, 152.0, 180.0, 26.0, 14.0, argb(C_SECONDARY), false);
                        let label = hotkey_label(&self.settings);
                        canvas.text(&label, 24.0, 178.0, 332.0, 34.0, 19.0, argb(C_TEXT), true);
                    }
                    let fill = if hover == ID_HOTKEY { argb(C_HOVER) } else { argb(C_TRACK) };
                    canvas.round_rect(24.0, 224.0, (PANEL_W - 48) as f32, 40.0, 10.0, fill, argb(C_BORDER), 1.0);
                    canvas.text(text(lang, "hotkey_change"), 24.0, 224.0, (PANEL_W - 48) as f32, 40.0, 15.0, argb(C_TEXT), true);
                    canvas.text(text(lang, "hotkey_hint1"), 24.0, 284.0, 332.0, 26.0, 13.0, argb(C_SECONDARY), true);
                    canvas.text(text(lang, "hotkey_hint2"), 24.0, 308.0, 332.0, 26.0, 13.0, argb(C_SECONDARY), true);
                }
                3 => {
                    canvas.text(text(lang, "display_title"), 24.0, 96.0, 332.0, 30.0, 17.0, argb(C_TEXT), false);

                    canvas.text(text(lang, "display_duration"), 24.0, 126.0, 240.0, 24.0, 14.0, argb(C_SECONDARY), false);
                    let idx = slider_index(&self.settings, SLIDER_DURATION);
                    let value = slider_value_text(&self.settings, lang, SLIDER_DURATION);
                    paint_slider(
                        &mut canvas, DURATION_TRACK_Y, SLIDER_DURATION, idx, &value,
                        hover == ID_DURATION_SLIDER || self.panel.drag_slider == SLIDER_DURATION,
                    );

                    canvas.text(text(lang, "panel_opacity"), 24.0, 192.0, 240.0, 24.0, 14.0, argb(C_SECONDARY), false);
                    let idx = slider_index(&self.settings, SLIDER_PANEL_OP);
                    let value = slider_value_text(&self.settings, lang, SLIDER_PANEL_OP);
                    paint_slider(
                        &mut canvas, PANEL_OP_TRACK_Y, SLIDER_PANEL_OP, idx, &value,
                        hover == ID_PANEL_SLIDER || self.panel.drag_slider == SLIDER_PANEL_OP,
                    );

                    canvas.text(text(lang, "overlay_opacity"), 24.0, 258.0, 240.0, 24.0, 14.0, argb(C_SECONDARY), false);
                    let idx = slider_index(&self.settings, SLIDER_OVERLAY_OP);
                    let value = slider_value_text(&self.settings, lang, SLIDER_OVERLAY_OP);
                    paint_slider(
                        &mut canvas, OVERLAY_OP_TRACK_Y, SLIDER_OVERLAY_OP, idx, &value,
                        hover == ID_OVERLAY_SLIDER || self.panel.drag_slider == SLIDER_OVERLAY_OP,
                    );

                    canvas.text(text(lang, "sound_volume"), 24.0, 324.0, 240.0, 24.0, 14.0, argb(C_SECONDARY), false);
                    let idx = slider_index(&self.settings, SLIDER_SOUND);
                    let value = slider_value_text(&self.settings, lang, SLIDER_SOUND);
                    paint_slider(
                        &mut canvas, SOUND_VOLUME_TRACK_Y, SLIDER_SOUND, idx, &value,
                        hover == ID_SOUND_SLIDER || self.panel.drag_slider == SLIDER_SOUND,
                    );

                    let fill = if hover == ID_POSITION { argb(C_HOVER) } else { argb(C_TRACK) };
                    canvas.round_rect(24.0, 382.0, (PANEL_W - 48) as f32, 34.0, 10.0, fill, argb(C_BORDER), 1.0);
                    canvas.text(text(lang, "position_button"), 24.0, 382.0, (PANEL_W - 48) as f32, 34.0, 15.0, argb(C_TEXT), true);
                }
                _ => {}
            }

            let qfill = if hover == ID_QUIT { argb(C_HOVER) } else { argb(C_TRACK) };
            canvas.round_rect(24.0, 444.0, (PANEL_W - 48) as f32, 40.0, 10.0, qfill, argb(C_BORDER), 1.0);
            canvas.text(text(lang, "quit"), 24.0, 444.0, (PANEL_W - 48) as f32, 40.0, 15.0, argb(C_TEXT), true);

            // Language dropdown is painted last so nothing from the active
            // view can show through it: the card is fully opaque.
            if self.panel.lang_open {
                canvas.round_rect(
                    LANG_MENU_X as f32, LANG_MENU_Y as f32,
                    LANG_MENU_W as f32, LANG_MENU_H as f32, 14.0,
                    argb(C_BG), argb(C_BORDER), 1.0,
                );
                canvas.text(
                    text(lang, "lang_label"), (LANG_MENU_X + 16) as f32,
                    LANG_MENU_Y as f32 + LANG_MENU_PAD as f32,
                    (LANG_MENU_W - 32) as f32, LANG_MENU_HEADER_H as f32,
                    11.0, argb(C_SECONDARY), false,
                );
                let divider_y = (LANG_MENU_Y + LANG_MENU_PAD + LANG_MENU_HEADER_H - 2) as f32;
                canvas.line(
                    (LANG_MENU_X + 12) as f32, divider_y,
                    (LANG_MENU_X + LANG_MENU_W - 12) as f32, divider_y,
                    argb(C_BORDER), 1.0,
                );
                for (i, &li) in LANGS.iter().enumerate() {
                    let (cx, cy, cw, ch) = lang_cell_rect(i);
                    let selected = li == lang;
                    let (fill, border, border_w) = if selected {
                        (argb(C_SELECT), argb(C_ACCENT), 1.0)
                    } else if hover == ID_LANG_BASE + i as i32 {
                        (argb(C_HOVER), 0, 0.0)
                    } else {
                        (0, 0, 0.0)
                    };
                    canvas.round_rect(
                        (cx + 2) as f32, (cy + 2) as f32,
                        (cw - 4) as f32, (ch - 4) as f32,
                        7.0, fill, border, border_w,
                    );
                    canvas.set_language(li);
                    canvas.text(
                        li.native(), (cx + 12) as f32, (cy + 2) as f32,
                        (cw - 44) as f32, (ch - 4) as f32,
                        13.0, argb(C_TEXT), false,
                    );
                    canvas.set_language(lang);
                    if selected {
                        canvas.ellipse(
                            (cx + cw - 18) as f32, (cy + ch / 2 - 4) as f32,
                            8.0, 8.0, argb(C_ACCENT), 0, 0.0,
                        );
                    }
                }
                canvas.set_language(lang);
            }

            // Burger dropdown: opaque floating card, same dark grey as the
            // panel, with a dark-green selection bar instead of a bright fill.
            if self.panel.menu_open {
                canvas.round_rect(
                    MENU_X as f32, MENU_Y as f32, MENU_W as f32, MENU_H as f32, 14.0,
                    argb(C_BG), argb(C_BORDER), 1.0,
                );
                canvas.text(
                    text(lang, "menu_categories"), (MENU_X + 16) as f32,
                    (MENU_Y + MENU_PAD) as f32, (MENU_W - 32) as f32, MENU_HEADER_H as f32,
                    11.0, argb(C_SECONDARY), false,
                );
                let divider_y = (MENU_Y + MENU_PAD + MENU_HEADER_H - 2) as f32;
                canvas.line(
                    (MENU_X + 12) as f32, divider_y,
                    (MENU_X + MENU_W - 12) as f32, divider_y,
                    argb(C_BORDER), 1.0,
                );
                for (i, key) in CATEGORY_KEYS.iter().enumerate() {
                    let ry = MENU_Y + MENU_PAD + MENU_HEADER_H + i as i32 * MENU_ROW_H;
                    let id = ID_CATEGORY_BASE + i as i32;
                    let selected = view == i as u8;
                    let fill = if selected {
                        argb(C_SELECT)
                    } else if hover == id {
                        argb(C_HOVER)
                    } else {
                        0
                    };
                    canvas.round_rect(
                        (MENU_X + 8) as f32, (ry + 3) as f32,
                        (MENU_W - 16) as f32, (MENU_ROW_H - 6) as f32,
                        9.0, fill,
                        if selected { argb(C_ACCENT) } else { 0 },
                        if selected { 1.0 } else { 0.0 },
                    );
                    if selected {
                        canvas.round_rect(
                            (MENU_X + 14) as f32, (ry + 10) as f32,
                            3.0, (MENU_ROW_H - 20) as f32, 1.5,
                            argb(C_ACCENT), 0, 0.0,
                        );
                        canvas.ellipse(
                            (MENU_X + MENU_W - 26) as f32,
                            (ry + MENU_ROW_H / 2 - 4) as f32,
                            8.0, 8.0, argb(C_ACCENT), 0, 0.0,
                        );
                    }
                    canvas.text(
                        text(lang, key), (MENU_X + 26) as f32, (ry + 3) as f32,
                        (MENU_W - 64) as f32, (MENU_ROW_H - 6) as f32, 14.0,
                        argb(C_TEXT), false,
                    );
                }
            }
        }

        surface.premultiply();
        let (x, y) = panel_pos();
        let screen = GetDC(core::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let old = SelectObject(mem, surface.bitmap);
        let pos = POINT { x, y };
        let size = SIZE { cx: PANEL_W, cy: PANEL_H };
        let src = POINT { x: 0, y: 0 };
        let blend_fn = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER,
            BlendFlags: 0,
            SourceConstantAlpha: (self.settings.panel_opacity as i32 * 255 / 100).clamp(0, 255) as u8,
            AlphaFormat: AC_SRC_ALPHA,
        };
        UpdateLayeredWindow(
            self.hwnd_panel, screen, &pos, &size, mem, &src, 0, &blend_fn, ULW_ALPHA,
        );
        SelectObject(mem, old);
        DeleteDC(mem);
        ReleaseDC(core::ptr::null_mut(), screen);
    }

    unsafe fn paint_panel_gdi(&mut self, hwnd: HWND) {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mem = CreateCompatibleDC(hdc);
        let bmp = CreateCompatibleBitmap(hdc, PANEL_W, PANEL_H);
        let old_bmp = SelectObject(mem, bmp);
        let lang = self.settings.language;
        let view = self.panel.view;
        let hover = self.panel.hover;

        let bg = CreateSolidBrush(C_BG);
        let rect = RECT { left: 0, top: 0, right: PANEL_W, bottom: PANEL_H };
        FillRect(mem, &rect, bg);
        DeleteObject(bg);

        let brush = CreateSolidBrush(C_BG);
        let pen = CreatePen(PS_SOLID, 1, C_BORDER);
        let ob = SelectObject(mem, brush);
        let op = SelectObject(mem, pen);
        RoundRect(mem, 1, 1, PANEL_W - 1, PANEL_H - 1, 34, 34);
        SelectObject(mem, ob);
        SelectObject(mem, op);
        DeleteObject(brush);
        DeleteObject(pen);

        let sep = CreatePen(PS_SOLID, 1, C_BORDER);
        let op = SelectObject(mem, sep);
        MoveToEx(mem, 24, 84, core::ptr::null_mut());
        LineTo(mem, PANEL_W - 24, 84);
        SelectObject(mem, op);
        DeleteObject(sep);

        let dpi = self.panel.dpi;
        let gfx_ref = self.gfx.as_ref();
        let paint_text_for = |label: &str, x: i32, y: i32, w: i32, h: i32, pt: f32, color: u32, centered: bool, text_lang: Lang| unsafe {
            if let Some(gfx) = gfx_ref {
                gfx.draw_text(mem, label, x as f32, y as f32, w as f32, h as f32, pt, color, centered, text_lang);
            } else {
                let font = make_font(pt, dpi, false);
                let mut r = RECT { left: x, top: y, right: x + w, bottom: y + h };
                let flags = if centered {
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX
                } else {
                    DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX
                };
                draw_text(mem, label, &mut r, font, color, flags);
                DeleteObject(font);
            }
        };
        let paint_text = |label: &str, x: i32, y: i32, w: i32, h: i32, pt: f32, color: u32, centered: bool| {
            paint_text_for(label, x, y, w, h, pt, color, centered, lang)
        };

        let header_icon = LoadIconW(self.hinstance, IDI_LOGO as *const u16);
        if !header_icon.is_null() {
            // DI_IMAGE | DI_MASK; the icon stays owned by the module.
            DrawIconEx(
                mem, HEADER_LOGO_X, HEADER_LOGO_Y, header_icon,
                HEADER_LOGO_SIZE, HEADER_LOGO_SIZE, 0, core::ptr::null_mut(), 0x0003,
            );
        }
        paint_text(
            "IndiCaps", HEADER_TITLE_X, HEADER_TITLE_Y + 4,
            HEADER_TITLE_W, HEADER_TITLE_H - 4, 17.0, C_TEXT, false,
        );
        paint_text(
            text(lang, CATEGORY_KEYS[view as usize]),
            HEADER_SUBTITLE_X, HEADER_SUBTITLE_Y + 2,
            HEADER_SUBTITLE_W, HEADER_SUBTITLE_H - 6, 11.0, C_SECONDARY, false,
        );

        let burger_active = self.panel.menu_open || hover == ID_MENU;
        self.rounded(
            mem, BURGER_X, BURGER_Y, BURGER_W, BURGER_H, 8,
            if burger_active { C_SELECT } else { C_TRACK },
            if burger_active { C_ACCENT } else { C_BORDER },
        );
        let icon = if burger_active { C_ACCENT } else { C_TEXT };
        for (i, &len) in [16i32, 12, 16].iter().enumerate() {
            let lx = BURGER_X + (BURGER_W - len) / 2;
            let ly = BURGER_Y + 8 + i as i32 * 6;
            self.rounded(mem, lx, ly, len, 2, 1, icon, icon);
        }

        self.rounded(mem, CLOSE_X, CLOSE_Y, CLOSE_W, CLOSE_H, 8,
            if hover == ID_CLOSE { C_HOVER } else { C_TRACK }, C_BORDER);
        let xpen = CreatePen(PS_SOLID, 2, C_TEXT);
        let op = SelectObject(mem, xpen);
        MoveToEx(mem, CLOSE_X + 9, CLOSE_Y + 9, core::ptr::null_mut());
        LineTo(mem, CLOSE_X + CLOSE_W - 9, CLOSE_Y + CLOSE_H - 9);
        MoveToEx(mem, CLOSE_X + CLOSE_W - 9, CLOSE_Y + 9, core::ptr::null_mut());
        LineTo(mem, CLOSE_X + 9, CLOSE_Y + CLOSE_H - 9);
        SelectObject(mem, op);
        DeleteObject(xpen);

        match view {
            0 => {
                paint_text(text(lang, "lang_label"), 24, LANG_LABEL_Y, 200, 22, 11.5, C_SECONDARY, false);

                let lang_open = self.panel.lang_open;
                let (btn_fill, btn_border) = if lang_open {
                    (C_SELECT, C_ACCENT)
                } else if hover == ID_LANG_SELECT {
                    (C_HOVER, C_BORDER)
                } else {
                    (C_TRACK, C_BORDER)
                };
                self.rounded(mem, LANG_BTN_X, LANG_BTN_Y, LANG_BTN_W, LANG_BTN_H, 10, btn_fill, btn_border);
                paint_text_for(
                    lang.native(), LANG_BTN_X + 14, LANG_BTN_Y, LANG_BTN_W - 52, LANG_BTN_H,
                    11.0, C_TEXT, false, lang,
                );
                let chevron = CreatePen(PS_SOLID, 2, if lang_open { C_ACCENT } else { C_SECONDARY });
                let cop = SelectObject(mem, chevron);
                let chx = LANG_BTN_X + LANG_BTN_W - 26;
                let chy = LANG_BTN_Y + LANG_BTN_H / 2;
                if lang_open {
                    MoveToEx(mem, chx - 5, chy + 2, core::ptr::null_mut());
                    LineTo(mem, chx, chy - 3);
                    LineTo(mem, chx + 5, chy + 2);
                } else {
                    MoveToEx(mem, chx - 5, chy - 2, core::ptr::null_mut());
                    LineTo(mem, chx, chy + 3);
                    LineTo(mem, chx + 5, chy - 2);
                }
                SelectObject(mem, cop);
                DeleteObject(chevron);

                let values = [
                    self.settings.indicator,
                    self.settings.num_indicator,
                    self.settings.sound,
                    self.settings.animation,
                    self.settings.startup,
                ];
                for i in 0..OPT_COUNT {
                    let y = OPT_ROW_Y + i as i32 * OPT_ROW_STEP;
                    paint_text(text(lang, OPT_KEYS[i]), 24, y, 270, 34, 11.5, C_TEXT, false);
                    let sx = PANEL_W - 70;
                    let sy = y + 5;
                    let track = if values[i] { C_ACCENT } else { C_TRACK };
                    self.rounded(mem, sx, sy, 46, 26, 13, track, track);
                    let knob = self.panel.knob[i].clamp(0.0, 1.0);
                    let kx = sx + 3 + (knob * 20.0) as i32;
                    let brush = CreateSolidBrush(C_TEXT);
                    let pen = CreatePen(PS_SOLID, 1, C_TEXT);
                    let ob = SelectObject(mem, brush);
                    let op = SelectObject(mem, pen);
                    Ellipse(mem, kx, sy + 3, kx + 20, sy + 23);
                    SelectObject(mem, ob);
                    SelectObject(mem, op);
                    DeleteObject(brush);
                    DeleteObject(pen);
                }
            }
            1 => {
                paint_text(text(lang, "stats_title"), 24, 110, 332, 30, 12.0, C_TEXT, false);
                let rows = [
                    (text(lang, "stat_caps"), self.settings.caps_toggles.to_string()),
                    (text(lang, "stat_num"), self.settings.num_toggles.to_string()),
                    (text(lang, "stat_caps_time"), format_duration(self.settings.caps_on_ms)),
                    (text(lang, "stat_num_time"), format_duration(self.settings.num_on_ms)),
                    (text(lang, "stat_since"), stats_since(&self.settings)),
                ];
                for (i, (key, value)) in rows.iter().enumerate() {
                    let y = 146 + i as i32 * 38;
                    paint_text(key, 24, y, 180, 28, 10.5, C_SECONDARY, false);
                    paint_text(value, 204, y, 152, 28, 11.0, C_TEXT, true);
                }
                if self.panel.reset_confirm {
                    paint_text(text(lang, "stat_reset_confirm"), 24, 334, 332, 26, 9.5, C_ACCENT, true);
                    let no_fill = if hover == ID_RESET_NO { C_HOVER } else { C_TRACK };
                    self.rounded(mem, 24, 370, 160, 34, 9, no_fill, no_fill);
                    paint_text(text(lang, "stat_reset_no"), 24, 370, 160, 34, 10.5, C_TEXT, true);
                    self.rounded(mem, 196, 370, 160, 34, 9, C_ACCENT, C_ACCENT);
                    paint_text(text(lang, "stat_reset_yes"), 196, 370, 160, 34, 10.5, C_BG, true);
                } else {
                    paint_text(text(lang, "stat_local"), 24, 334, 332, 26, 9.5, C_SECONDARY, true);
                    let fill = if hover == ID_RESET { C_HOVER } else { C_TRACK };
                    self.rounded(mem, 24, 370, PANEL_W - 48, 34, 9, fill, fill);
                    paint_text(text(lang, "stat_reset"), 24, 370, PANEL_W - 48, 34, 10.5, C_TEXT, true);
                }
            }
            2 => {
                paint_text(text(lang, "hotkey_title"), 24, 110, 332, 30, 12.0, C_TEXT, false);
                if self.panel.hotkey_capturing {
                    paint_text(text(lang, "hotkey_wait"), 24, 156, 332, 34, 12.0, C_ACCENT, true);
                    paint_text(text(lang, "hotkey_cancel"), 24, 194, 332, 26, 10.0, C_SECONDARY, true);
                } else {
                    paint_text(text(lang, "hotkey_current"), 24, 152, 180, 26, 10.5, C_SECONDARY, false);
                    let label = hotkey_label(&self.settings);
                    paint_text(&label, 24, 178, 332, 34, 13.0, C_TEXT, true);
                }
                let fill = if hover == ID_HOTKEY { C_HOVER } else { C_TRACK };
                self.rounded(mem, 24, 224, PANEL_W - 48, 40, 10, fill, C_BORDER);
                paint_text(text(lang, "hotkey_change"), 24, 224, PANEL_W - 48, 40, 11.0, C_TEXT, true);
                paint_text(text(lang, "hotkey_hint1"), 24, 284, 332, 26, 10.0, C_SECONDARY, true);
                paint_text(text(lang, "hotkey_hint2"), 24, 308, 332, 26, 10.0, C_SECONDARY, true);
            }
            3 => {
                paint_text(text(lang, "display_title"), 24, 96, 332, 30, 12.0, C_TEXT, false);
                let paint_slider = |y: i32, kind: u8, idx: usize, value: &str, active: bool| {
                    let steps = slider_steps(kind) as i32;
                    let usable = SLIDER_W - 18;
                    let cy = y + SLIDER_TRACK_H / 2;
                    let center = SLIDER_X + 9 + usable * idx as i32 / (steps - 1);
                    self.rounded(mem, SLIDER_X, y, SLIDER_W, SLIDER_TRACK_H, 5, C_TRACK, C_BORDER);
                    if idx > 0 {
                        self.rounded(mem, SLIDER_X, y, center - SLIDER_X, SLIDER_TRACK_H, 5, C_ACCENT, C_ACCENT);
                    }
                    let brush = CreateSolidBrush(C_TEXT);
                    let old_brush = SelectObject(mem, brush);
                    for i in 0..steps {
                        let tx = SLIDER_X + 9 + usable * i / (steps - 1);
                        Ellipse(mem, tx - 2, cy - 2, tx + 2, cy + 2);
                    }
                    SelectObject(mem, old_brush);
                    DeleteObject(brush);
                    let brush = CreateSolidBrush(C_TEXT);
                    let old_brush = SelectObject(mem, brush);
                    Ellipse(mem, center - 9, cy - 9, center + 9, cy + 9);
                    SelectObject(mem, old_brush);
                    DeleteObject(brush);
                    let (fill, border) = if active {
                        (C_SELECT, C_ACCENT)
                    } else {
                        (C_TRACK, C_BORDER)
                    };
                    self.rounded(mem, SLIDER_VAL_X, y - 11, SLIDER_VAL_W, 32, 10, fill, border);
                    paint_text(value, SLIDER_VAL_X, y - 11, SLIDER_VAL_W, 32, 10.5,
                        C_TEXT, true);
                };

                let idx = slider_index(&self.settings, SLIDER_DURATION);
                let value = slider_value_text(&self.settings, lang, SLIDER_DURATION);
                paint_text(text(lang, "display_duration"), 24, 126, 240, 24, 10.5, C_SECONDARY, false);
                paint_slider(DURATION_TRACK_Y, SLIDER_DURATION, idx, &value,
                    hover == ID_DURATION_SLIDER || self.panel.drag_slider == SLIDER_DURATION);

                let idx = slider_index(&self.settings, SLIDER_PANEL_OP);
                let value = slider_value_text(&self.settings, lang, SLIDER_PANEL_OP);
                paint_text(text(lang, "panel_opacity"), 24, 192, 240, 24, 10.5, C_TEXT, false);
                paint_slider(PANEL_OP_TRACK_Y, SLIDER_PANEL_OP, idx, &value,
                    hover == ID_PANEL_SLIDER || self.panel.drag_slider == SLIDER_PANEL_OP);

                let idx = slider_index(&self.settings, SLIDER_OVERLAY_OP);
                let value = slider_value_text(&self.settings, lang, SLIDER_OVERLAY_OP);
                paint_text(text(lang, "overlay_opacity"), 24, 258, 240, 24, 10.5, C_TEXT, false);
                paint_slider(OVERLAY_OP_TRACK_Y, SLIDER_OVERLAY_OP, idx, &value,
                    hover == ID_OVERLAY_SLIDER || self.panel.drag_slider == SLIDER_OVERLAY_OP);

                let idx = slider_index(&self.settings, SLIDER_SOUND);
                let value = slider_value_text(&self.settings, lang, SLIDER_SOUND);
                paint_text(text(lang, "sound_volume"), 24, 324, 240, 24, 10.5, C_SECONDARY, false);
                paint_slider(SOUND_VOLUME_TRACK_Y, SLIDER_SOUND, idx, &value,
                    hover == ID_SOUND_SLIDER || self.panel.drag_slider == SLIDER_SOUND);

                let fill = if hover == ID_POSITION { C_HOVER } else { C_TRACK };
                self.rounded(mem, 24, 382, PANEL_W - 48, 34, 10, fill, C_BORDER);
                paint_text(text(lang, "position_button"), 24, 382, PANEL_W - 48, 34, 11.0, C_TEXT, true);
            }
            _ => {}
        }

        let qfill = if hover == ID_QUIT { C_HOVER } else { C_TRACK };
        self.rounded(mem, 24, 444, PANEL_W - 48, 40, 10, qfill, C_BORDER);
        paint_text(text(lang, "quit"), 24, 444, PANEL_W - 48, 40, 11.0, C_TEXT, true);

        // Painted last: the language card covers every control behind it.
        if self.panel.lang_open {
            self.rounded(mem, LANG_MENU_X, LANG_MENU_Y, LANG_MENU_W, LANG_MENU_H, 14, C_BG, C_BORDER);
            paint_text_for(
                text(lang, "lang_label"), LANG_MENU_X + 16, LANG_MENU_Y + LANG_MENU_PAD,
                LANG_MENU_W - 32, LANG_MENU_HEADER_H, 9.5, C_SECONDARY, false, lang,
            );
            let div = CreatePen(PS_SOLID, 1, C_BORDER);
            let dop = SelectObject(mem, div);
            let divider_y = LANG_MENU_Y + LANG_MENU_PAD + LANG_MENU_HEADER_H - 2;
            MoveToEx(mem, LANG_MENU_X + 12, divider_y, core::ptr::null_mut());
            LineTo(mem, LANG_MENU_X + LANG_MENU_W - 12, divider_y);
            SelectObject(mem, dop);
            DeleteObject(div);

            for (i, &li) in LANGS.iter().enumerate() {
                let (cx, cy, cw, ch) = lang_cell_rect(i);
                let selected = li == lang;
                let (fill, border) = if selected {
                    (C_SELECT, C_ACCENT)
                } else if hover == ID_LANG_BASE + i as i32 {
                    (C_HOVER, C_HOVER)
                } else {
                    (C_BG, C_BG)
                };
                self.rounded(mem, cx + 2, cy + 2, cw - 4, ch - 4, 7, fill, border);
                paint_text_for(
                    li.native(), cx + 12, cy + 2, cw - 44, ch - 4,
                    9.5, C_TEXT, false, li,
                );
                if selected {
                    let brush = CreateSolidBrush(C_ACCENT);
                    let ob = SelectObject(mem, brush);
                    Ellipse(mem, cx + cw - 18, cy + ch / 2 - 4, cx + cw - 10, cy + ch / 2 + 4);
                    SelectObject(mem, ob);
                    DeleteObject(brush);
                }
            }
        }

        if self.panel.menu_open {
            self.rounded(mem, MENU_X, MENU_Y, MENU_W, MENU_H, 14, C_BG, C_BORDER);
            paint_text(text(lang, "menu_categories"), MENU_X + 16, MENU_Y + MENU_PAD,
                MENU_W - 32, MENU_HEADER_H, 9.5, C_SECONDARY, false);
            let div = CreatePen(PS_SOLID, 1, C_BORDER);
            let dop = SelectObject(mem, div);
            let divider_y = MENU_Y + MENU_PAD + MENU_HEADER_H - 2;
            MoveToEx(mem, MENU_X + 12, divider_y, core::ptr::null_mut());
            LineTo(mem, MENU_X + MENU_W - 12, divider_y);
            SelectObject(mem, dop);
            DeleteObject(div);

            for (i, key) in CATEGORY_KEYS.iter().enumerate() {
                let ry = MENU_Y + MENU_PAD + MENU_HEADER_H + i as i32 * MENU_ROW_H;
                let id = ID_CATEGORY_BASE + i as i32;
                let selected = view == i as u8;
                let (fill, border) = if selected {
                    (C_SELECT, C_ACCENT)
                } else if hover == id {
                    (C_HOVER, C_HOVER)
                } else {
                    (C_BG, C_BG)
                };
                self.rounded(mem, MENU_X + 8, ry + 3, MENU_W - 16, MENU_ROW_H - 6,
                    9, fill, border);
                if selected {
                    self.rounded(mem, MENU_X + 14, ry + 10, 3, MENU_ROW_H - 20,
                        1, C_ACCENT, C_ACCENT);
                    let brush = CreateSolidBrush(C_ACCENT);
                    let ob = SelectObject(mem, brush);
                    Ellipse(mem, MENU_X + MENU_W - 26, ry + MENU_ROW_H / 2 - 4,
                        MENU_X + MENU_W - 18, ry + MENU_ROW_H / 2 + 4);
                    SelectObject(mem, ob);
                    DeleteObject(brush);
                }
                paint_text(text(lang, key), MENU_X + 26, ry + 3,
                    MENU_W - 64, MENU_ROW_H - 6, 10.5, C_TEXT, false);
            }
        }

        BitBlt(hdc, 0, 0, PANEL_W, PANEL_H, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old_bmp);
        DeleteObject(bmp);
        DeleteDC(mem);
        EndPaint(hwnd, &ps);
    }

    unsafe fn rounded(&self, dc: HDC, x: i32, y: i32, w: i32, h: i32, r: i32, fill: u32, border: u32) {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID, 1, border);
        let ob = SelectObject(dc, brush);
        let op = SelectObject(dc, pen);
        RoundRect(dc, x, y, x + w, y + h, r * 2, r * 2);
        SelectObject(dc, ob);
        SelectObject(dc, op);
        DeleteObject(brush);
        DeleteObject(pen);
    }

    unsafe fn msg_proc(&mut self, hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            WM_APP_TRAY => {
                match lp as u32 {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => self.toggle_panel(),
                    WM_RBUTTONUP => self.show_tray_menu(),
                    _ => {}
                }
                0
            }
            WM_APP_CAPS => {
                self.on_caps_change();
                0
            }
            WM_APP_NUM => {
                self.on_num_change();
                0
            }
            WM_APP_HOTKEY => {
                self.log("hotkey triggered");
                if !self.pos_open {
                    self.toggle_panel();
                }
                0
            }
            WM_APP_REFRESH => {
                self.invalidate_panel();
                0
            }
            WM_APP_HOTKEY_STORED => {
                self.log("hotkey captured");
                self.settings.save();
                self.invalidate_panel();
                0
            }
            WM_TIMER => {
                self.on_timer(wp);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }

    unsafe fn overlay_proc(&self, hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        DefWindowProcW(hwnd, msg, wp, lp)
    }

    unsafe fn panel_proc(&mut self, hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            WM_SETCURSOR => {
                // Layered tool windows otherwise show the busy hourglass when
                // the pointer enters their client area.
                SetCursor(LoadCursorW(core::ptr::null_mut(), IDC_ARROW));
                1
            }
            WM_PAINT => {
                if self.gfx.is_some() {
                    // The layered panel is drawn through UpdateLayeredWindow;
                    // just validate the paint region without GDI drawing.
                    let mut ps = PAINTSTRUCT::default();
                    BeginPaint(hwnd, &mut ps);
                    EndPaint(hwnd, &ps);
                } else {
                    self.paint_panel_gdi(hwnd);
                }
                0
            }
            WM_LBUTTONDOWN => {
                let id = self.panel.hit(lo(lp), hi(lp));
                let kind = slider_kind(id);
                if kind != 0 {
                    self.panel.drag_slider = kind;
                    SetCapture(hwnd);
                }
                self.panel_click(lo(lp), hi(lp));
                0
            }
            WM_MOUSEMOVE => {
                if self.panel.drag_slider != 0 {
                    self.panel_slider_drag(lo(lp));
                } else {
                    let id = self.panel.hit(lo(lp), hi(lp));
                    if id != self.panel.hover {
                        self.panel.hover = id;
                        self.invalidate_panel();
                    }
                }
                if !self.panel.tracking {
                    self.panel.tracking = true;
                    let mut ev = TRACKMOUSEEVENT {
                        cbSize: core::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut ev);
                }
                0
            }
            WM_LBUTTONUP => {
                if self.panel.drag_slider != 0 {
                    ReleaseCapture();
                    self.apply_settings();
                    self.panel.drag_slider = 0;
                    self.invalidate_panel();
                }
                0
            }
            WM_MOUSELEAVE => {
                self.panel.tracking = false;
                if self.panel.drag_slider == 0 {
                    self.panel.hover = 0;
                    self.invalidate_panel();
                }
                0
            }
            WM_APP_REFRESH => {
                self.invalidate_panel();
                0
            }
            WM_KEYDOWN if wp == VK_ESCAPE as usize => {
                if self.panel.lang_open {
                    self.panel.lang_open = false;
                    self.invalidate_panel();
                } else if self.panel.menu_open {
                    self.panel.menu_open = false;
                    self.invalidate_panel();
                } else if self.panel.hotkey_capturing {
                    self.panel.hotkey_capturing = false;
                    self.invalidate_panel();
                } else if self.panel.reset_confirm {
                    self.panel.reset_confirm = false;
                    self.invalidate_panel();
                } else {
                    self.hide_panel();
                }
                0
            }
            WM_ACTIVATEAPP if wp == 0 => {
                if !self.panel.hotkey_capturing {
                    self.hide_panel();
                }
                0
            }
            WM_CLOSE => {
                self.hide_panel();
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }

    /// The preview window is transparent to hit-testing; all input still goes
    /// to the full-screen position editor underneath it.
    unsafe fn ghost_proc(&mut self, hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                BeginPaint(hwnd, &mut ps);
                EndPaint(hwnd, &ps);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }

    unsafe fn pos_proc(&mut self, hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            WM_SETCURSOR => {
                SetCursor(LoadCursorW(core::ptr::null_mut(), IDC_ARROW));
                1
            }
            WM_PAINT => {
                // The editor is drawn through UpdateLayeredWindow.
                let mut ps = PAINTSTRUCT::default();
                BeginPaint(hwnd, &mut ps);
                EndPaint(hwnd, &ps);
                0
            }
            WM_LBUTTONDOWN => {
                let (left, top, _, _) = work_area();
                let (w, h) = overlay_size(self.pos.dpi);
                self.pos.mouse_down(lo(lp), hi(lp), left, top, w, h);
                SetCapture(hwnd);
                self.invalidate_pos();
                0
            }
            WM_MOUSEMOVE => {
                if self.pos.dragging {
                    let (left, top, _, _) = work_area();
                    let (w, h) = overlay_size(self.pos.dpi);
                    let before = self.pos.active(w, h);
                    self.pos.mouse_move(lo(lp), hi(lp), left, top, w, h);
                    // Only the small preview window moves on every mouse event.
                    // The full-screen backdrop is repainted just when the
                    // highlighted snap card changes.
                    self.position_ghost_window();
                    if self.pos.active(w, h) != before {
                        self.invalidate_pos();
                    }
                }
                0
            }
            WM_LBUTTONUP => {
                if self.pos.dragging {
                    ReleaseCapture();
                    let (x, y) = self.pos.mouse_up();
                    self.settings.overlay_x = x;
                    self.settings.overlay_y = y;
                    self.settings.save();
                    self.close_position_editor();
                }
                0
            }
            WM_KEYDOWN if wp == VK_ESCAPE as usize => {
                if self.pos.dragging {
                    ReleaseCapture();
                }
                self.close_position_editor();
                0
            }
            WM_KEYDOWN if wp == 0x0D => {
                // Enter validates the current ghost position.
                if self.pos.dragging {
                    ReleaseCapture();
                }
                let (x, y) = self.pos.mouse_up();
                self.settings.overlay_x = x;
                self.settings.overlay_y = y;
                self.settings.save();
                self.close_position_editor();
                0
            }
            WM_CLOSE => {
                self.close_position_editor();
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

unsafe fn create_tray_icon() -> HICON {
    const SIZE: i32 = 32;
    let surface = Surface::new(SIZE, SIZE);
    if !surface.is_valid() {
        return core::ptr::null_mut();
    }
    for y in 0..SIZE {
        for x in 0..SIZE {
            let cov = rounded_coverage(x - 2, y - 2, SIZE - 4, SIZE - 4, 7.0);
            if cov == 0 {
                continue;
            }
            let inner = rounded_coverage(x - 3, y - 3, SIZE - 6, SIZE - 6, 6.0);
            let col = mix([22, 22, 28], [94, 224, 132], (255 - inner) as f32 / 255.0);
            surface.put(x, y, [col[0], col[1], col[2], cov]);
        }
    }
    let mask = surface.text_mask("I", 10.5, 96, true);
    if mask.len() != (SIZE * SIZE) as usize {
        return core::ptr::null_mut();
    }
    for y in 0..SIZE {
        for x in 0..SIZE {
            let cov = mask[(y * SIZE + x) as usize];
            if cov > 0 {
                let mut px = surface.get(x, y);
                blend(&mut px, [255, 255, 255], cov);
                surface.put(x, y, px);
            }
        }
    }

    // CreateBitmap expects bottom-up BGRA rows.
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = surface.get(x, y);
            let dst = (((SIZE - 1 - y) * SIZE + x) * 4) as usize;
            pixels[dst..dst + 4].copy_from_slice(&px);
        }
    }
    let hbm_color = CreateBitmap(SIZE, SIZE, 1, 32, pixels.as_ptr() as *const core::ffi::c_void);
    let mask_bits = vec![0u8; (SIZE * SIZE / 8) as usize];
    let hbm_mask = CreateBitmap(
        SIZE,
        SIZE,
        1,
        1,
        mask_bits.as_ptr() as *const core::ffi::c_void,
    );
    let info = ICONINFO {
        fIcon: 1,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: hbm_mask,
        hbmColor: hbm_color,
    };
    let icon = CreateIconIndirect(&info);
    DeleteObject(hbm_mask);
    DeleteObject(hbm_color);
    icon
}

unsafe extern "system" fn hook_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // A nested modal loop (tray menu) can re-enter the hook while the outer
    // call still holds a `&mut App`. Bail out instead of aliasing it.
    if HOOK_BUSY.swap(true, Ordering::AcqRel) {
        return CallNextHookEx(core::ptr::null_mut(), code, wp, lp);
    }
    let result = hook_proc_inner(code, wp, lp);
    HOOK_BUSY.store(false, Ordering::Release);
    result
}

unsafe fn hook_proc_inner(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let app_ptr = APP_PTR;
    if app_ptr.is_null() {
        return CallNextHookEx(core::ptr::null_mut(), code, wp, lp);
    }
    let app = &mut *app_ptr;
    if code >= 0 {
        let data = &*(lp as *const KBDLLHOOKSTRUCT);
        let vk = data.vkCode;
        let down = wp == WM_KEYDOWN as usize || wp == WM_SYSKEYDOWN as usize;
        let up = wp == WM_KEYUP as usize || wp == WM_SYSKEYUP as usize;

        if let Some(bit) = modifier_bit(vk) {
            if down {
                app.mod_down |= bit;
            } else if up {
                app.mod_down &= !bit;
            }
        }

        if down && !is_modifier(vk) {
            if app.panel.hotkey_capturing {
                if vk == VK_ESCAPE as u32 {
                    app.panel.hotkey_capturing = false;
                    app.hotkey_down = false;
                    PostMessageW(app.hwnd_panel, WM_APP_REFRESH, 0, 0);
                    return 1;
                }
                if !is_toggle_key(vk) {
                    let mods = app.mod_down;
                    if mods != 0 {
                        app.settings.hotkey_mod = mods;
                        app.settings.hotkey_vk = vk;
                        app.hotkey_down = false;
                        app.panel.hotkey_capturing = false;
                        // No disk I/O inside a low-level keyboard hook: Windows
                        // can silently remove a hook that takes too long.
                        PostMessageW(app.hwnd_msg, WM_APP_HOTKEY_STORED, 0, 0);
                        PostMessageW(app.hwnd_panel, WM_APP_REFRESH, 0, 0);
                        return 1;
                    }
                }
            } else if vk == app.settings.hotkey_vk
                && app.mod_down == app.settings.hotkey_mod
                && !app.hotkey_down
            {
                app.hotkey_down = true;
                PostMessageW(app.hwnd_msg, WM_APP_HOTKEY, 0, 0);
                return 1;
            }
        }
        if up && vk == app.settings.hotkey_vk {
            app.hotkey_down = false;
        }

        // Never guess the state by toggling a cached boolean. Key events can be
        // missed or reordered (remote desktop, filtered keys, focus loss), so we
        // notify on both edges and let the handler read the real OS toggle bit.
        let (index, message) = match vk as i32 {
            VK_CAPITAL => (0, WM_APP_CAPS),
            VK_NUMLOCK => (1, WM_APP_NUM),
            _ => (-1, 0),
        };
        if index >= 0 {
            let index = index as usize;
            if down && !app.key_down[index] {
                app.key_down[index] = true;
                PostMessageW(app.hwnd_msg, message, 0, 0);
            } else if up {
                app.key_down[index] = false;
                PostMessageW(app.hwnd_msg, message, 0, 0);
            }
        }
    }
    CallNextHookEx(app.hook, code, wp, lp)
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(*(lp as *const *const CreateInfo));
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.app as isize);
        SetWindowLongPtrW(hwnd, 0, create.kind as isize);
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let kind = GetWindowLongPtrW(hwnd, 0) as u8;
    let app = &mut *ptr;
    match kind {
        KIND_MSG => app.msg_proc(hwnd, msg, wp, lp),
        KIND_OVERLAY => app.overlay_proc(hwnd, msg, wp, lp),
        KIND_PANEL => app.panel_proc(hwnd, msg, wp, lp),
        KIND_POS => app.pos_proc(hwnd, msg, wp, lp),
        KIND_GHOST => app.ghost_proc(hwnd, msg, wp, lp),
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn main() {
    unsafe {
        SetProcessDpiAwarenessContext(-4); // PER_MONITOR_AWARE_V2
        let mutex_name = wide(MUTEX_NAME);
        let _mutex = CreateMutexW(core::ptr::null(), 1, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        let hinstance = GetModuleHandleW(core::ptr::null());
        let mut app = App::new(hinstance);
        APP_PTR = &mut app;
        if app.init() {
            app.run();
        }
        app.shutdown();
        APP_PTR = core::ptr::null_mut();
    }
}
