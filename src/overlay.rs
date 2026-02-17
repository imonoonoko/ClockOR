use std::sync::{Arc, Mutex};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetMonitorInfoW,
    InvalidateRect, MonitorFromWindow, SelectObject, SetBkMode, SetTextColor, TextOutW,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, FF_SWISS, FW_BOLD, HBRUSH, HGDIOBJ,
    MONITORINFO, MONITOR_DEFAULTTOPRIMARY, OUT_TT_PRECIS, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetForegroundWindow,
    GetSystemMetrics, KillTimer, LoadCursorW, PostQuitMessage, RegisterClassW,
    SetLayeredWindowAttributes, SetTimer, SetWindowPos, ShowWindow, HWND_TOPMOST, IDC_ARROW,
    LWA_ALPHA, LWA_COLORKEY, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE,
    WM_DESTROY, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::config::{Config, Position, TextStyle};

const TIMER_ID: usize = 1;
const CLASS_NAME: PCWSTR = w!("ClockOR_Overlay");
/// Color key for transparent background (RGB 1,0,1 — nearly black, won't match text)
const COLOR_KEY: COLORREF = COLORREF(0x00010001);

static OVERLAY_CONFIG: std::sync::OnceLock<Arc<Mutex<Config>>> = std::sync::OnceLock::new();

/// If a COLORREF matches COLOR_KEY (0x00010001), nudge the blue channel to avoid transparency.
fn guard_color_key(cr: u32) -> u32 {
    if cr == COLOR_KEY.0 {
        cr ^ 0x00010000 // flip blue channel bit
    } else {
        cr
    }
}

pub struct Overlay {
    pub hwnd: HWND,
}

fn get_config() -> Config {
    OVERLAY_CONFIG
        .get()
        .map(|c| c.lock().unwrap().clone())
        .unwrap_or_default()
}

pub fn update_config(config: &Config) {
    if let Some(arc) = OVERLAY_CONFIG.get() {
        *arc.lock().unwrap() = config.clone();
    }
}

/// Get the monitor rect (left, top, width, height) for the given window.
/// Falls back to primary monitor if the window handle is invalid.
fn monitor_rect_for(hwnd: HWND) -> (i32, i32, i32, i32) {
    unsafe {
        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            let rc = info.rcMonitor;
            (rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top)
        } else {
            (
                0,
                0,
                GetSystemMetrics(SM_CXSCREEN),
                GetSystemMetrics(SM_CYSCREEN),
            )
        }
    }
}

fn calc_window_rect(config: &Config, monitor: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    let (mon_x, mon_y, mon_w, mon_h) = monitor;
    let font_px = config.font_size as i32;

    // Approximate character width: ~0.6 * font height for proportional font
    let char_w = (font_px as f32 * 0.6) as i32;
    let text_chars = match (config.format_24h, config.show_seconds) {
        (true, true) => 8,   // "HH:MM:SS"
        (true, false) => 5,  // "HH:MM"
        (false, true) => 11, // "HH:MM:SS AM"
        (false, false) => 8, // "HH:MM AM"
    };
    let text_w = char_w * text_chars;
    // Extra width for outline/shadow to prevent clipping
    let style_pad = match config.text_style {
        TextStyle::Outline | TextStyle::Shadow => 4,
        TextStyle::None => 0,
    };
    let win_w = text_w + 24 + style_pad;
    let win_h = font_px + 16;
    let margin = 10;

    let (x, y) = match config.position {
        Position::TopRight => (mon_x + mon_w - win_w - margin, mon_y + margin),
        Position::TopLeft => (mon_x + margin, mon_y + margin),
        Position::BottomRight => (
            mon_x + mon_w - win_w - margin,
            mon_y + mon_h - win_h - margin,
        ),
        Position::BottomLeft => (mon_x + margin, mon_y + mon_h - win_h - margin),
    };

    (x, y, win_w, win_h)
}

fn format_time(config: &Config) -> String {
    let now = chrono::Local::now();
    match (config.format_24h, config.show_seconds) {
        (true, true) => now.format("%H:%M:%S").to_string(),
        (true, false) => now.format("%H:%M").to_string(),
        (false, true) => now.format("%I:%M:%S %p").to_string(),
        (false, false) => now.format("%I:%M %p").to_string(),
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let config = get_config();

            // Fill entire window with color key (this area becomes transparent)
            let mut rc = windows::Win32::Foundation::RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            let key_brush = CreateSolidBrush(COLOR_KEY);
            let _ = FillRect(hdc, &rc, key_brush);
            let _ = DeleteObject(key_brush);

            // Create font
            let font = CreateFontW(
                config.font_size as i32,
                0,
                0,
                0,
                FW_BOLD.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_TT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                5, // CLEARTYPE_QUALITY
                (DEFAULT_PITCH.0 | FF_SWISS.0) as u32,
                w!("Segoe UI"),
            );
            let old_font = SelectObject(hdc, HGDIOBJ(font.0));
            SetBkMode(hdc, TRANSPARENT);

            let time_str = format_time(&config);
            let wide: Vec<u16> = time_str.encode_utf16().collect();
            let tx = 12;
            let ty = 8;

            // Resolve colors, guarding against COLOR_KEY collision
            let text_cr = guard_color_key(config.text_colorref());
            let outline_cr = guard_color_key(config.outline_colorref());

            match config.text_style {
                TextStyle::Outline => {
                    SetTextColor(hdc, COLORREF(outline_cr));
                    for &(dx, dy) in &[
                        (-1i32, -1i32), (0, -1), (1, -1),
                        (-1, 0),                  (1, 0),
                        (-1, 1),  (0, 1),  (1, 1),
                    ] {
                        let _ = TextOutW(hdc, tx + dx, ty + dy, &wide);
                    }
                    SetTextColor(hdc, COLORREF(text_cr));
                    let _ = TextOutW(hdc, tx, ty, &wide);
                }
                TextStyle::Shadow => {
                    SetTextColor(hdc, COLORREF(outline_cr));
                    let _ = TextOutW(hdc, tx + 2, ty + 2, &wide);
                    SetTextColor(hdc, COLORREF(text_cr));
                    let _ = TextOutW(hdc, tx, ty, &wide);
                }
                TextStyle::None => {
                    SetTextColor(hdc, COLORREF(text_cr));
                    let _ = TextOutW(hdc, tx, ty, &wide);
                }
            }

            SelectObject(hdc, old_font);
            let _ = DeleteObject(font);

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_TIMER => {
            let config = get_config();
            // Use overlay's own monitor (stays on the monitor where it was shown)
            let monitor = monitor_rect_for(hwnd);
            let (x, y, w, h) = calc_window_rect(&config, monitor);
            let alpha = (config.opacity as f32 / 100.0 * 255.0) as u8;
            let _ = SetLayeredWindowAttributes(hwnd, COLOR_KEY, alpha, LWA_COLORKEY | LWA_ALPHA);
            let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
            let _ = InvalidateRect(hwnd, None, true);
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(hwnd, TIMER_ID);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIMARY: (i32, i32, i32, i32) = (0, 0, 1920, 1080);
    const OFFSET: (i32, i32, i32, i32) = (1920, 0, 2560, 1440);

    fn test_config() -> Config {
        Config::default()
    }

    // --- calc_window_rect position tests ---

    #[test]
    fn top_right_position() {
        let cfg = test_config(); // default = TopRight
        let (x, y, w, _h) = calc_window_rect(&cfg, PRIMARY);
        assert_eq!(x, 1920 - w - 10);
        assert_eq!(y, 10);
    }

    #[test]
    fn top_left_position() {
        let mut cfg = test_config();
        cfg.position = Position::TopLeft;
        let (x, y, _, _) = calc_window_rect(&cfg, PRIMARY);
        assert_eq!(x, 10);
        assert_eq!(y, 10);
    }

    #[test]
    fn bottom_right_position() {
        let mut cfg = test_config();
        cfg.position = Position::BottomRight;
        let (x, y, w, h) = calc_window_rect(&cfg, PRIMARY);
        assert_eq!(x, 1920 - w - 10);
        assert_eq!(y, 1080 - h - 10);
    }

    #[test]
    fn bottom_left_position() {
        let mut cfg = test_config();
        cfg.position = Position::BottomLeft;
        let (x, y, _, h) = calc_window_rect(&cfg, PRIMARY);
        assert_eq!(x, 10);
        assert_eq!(y, 1080 - h - 10);
    }

    // --- multi-monitor offset ---

    #[test]
    fn multi_monitor_offset() {
        let mut cfg = test_config();
        cfg.position = Position::TopLeft;
        let (x, y, _, _) = calc_window_rect(&cfg, OFFSET);
        assert_eq!(x, 1920 + 10);
        assert_eq!(y, 10);
    }

    // --- font size affects window size ---

    #[test]
    fn larger_font_increases_window() {
        let mut small_cfg = test_config();
        small_cfg.font_size = 16;
        let (_, _, w_s, h_s) = calc_window_rect(&small_cfg, PRIMARY);

        let mut large_cfg = test_config();
        large_cfg.font_size = 30;
        let (_, _, w_l, h_l) = calc_window_rect(&large_cfg, PRIMARY);

        assert!(w_l > w_s);
        assert!(h_l > h_s);
    }

    // --- show_seconds affects width ---

    #[test]
    fn seconds_increases_width() {
        let mut no_sec = test_config();
        no_sec.show_seconds = false;
        let (_, _, w_no, _) = calc_window_rect(&no_sec, PRIMARY);

        let mut with_sec = test_config();
        with_sec.show_seconds = true;
        let (_, _, w_yes, _) = calc_window_rect(&with_sec, PRIMARY);

        assert!(w_yes > w_no);
    }

    // --- format_time structure ---

    #[test]
    fn format_time_24h_no_seconds() {
        let mut cfg = test_config();
        cfg.format_24h = true;
        cfg.show_seconds = false;
        let s = format_time(&cfg);
        // "HH:MM" — 5 chars
        assert_eq!(s.len(), 5);
        assert_eq!(&s[2..3], ":");
    }

    #[test]
    fn format_time_24h_with_seconds() {
        let mut cfg = test_config();
        cfg.format_24h = true;
        cfg.show_seconds = true;
        let s = format_time(&cfg);
        // "HH:MM:SS" — 8 chars
        assert_eq!(s.len(), 8);
        assert_eq!(&s[2..3], ":");
        assert_eq!(&s[5..6], ":");
    }

    #[test]
    fn format_time_12h_no_seconds() {
        let mut cfg = test_config();
        cfg.format_24h = false;
        cfg.show_seconds = false;
        let s = format_time(&cfg);
        // "HH:MM AM" — 8 chars
        assert_eq!(s.len(), 8);
        assert!(s.ends_with("AM") || s.ends_with("PM"));
    }

    #[test]
    fn format_time_12h_with_seconds() {
        let mut cfg = test_config();
        cfg.format_24h = false;
        cfg.show_seconds = true;
        let s = format_time(&cfg);
        // "HH:MM:SS AM" — 11 chars
        assert_eq!(s.len(), 11);
        assert!(s.ends_with("AM") || s.ends_with("PM"));
    }

    // --- guard_color_key ---

    #[test]
    fn guard_color_key_passes_normal_colors() {
        assert_eq!(guard_color_key(0x00FFFFFF), 0x00FFFFFF); // white
        assert_eq!(guard_color_key(0x00000000), 0x00000000); // black
        assert_eq!(guard_color_key(0x000000FF), 0x000000FF); // red
    }

    #[test]
    fn guard_color_key_nudges_matching_color() {
        // COLOR_KEY = 0x00010001, should be nudged
        assert_ne!(guard_color_key(0x00010001), 0x00010001);
        // Result should differ only slightly
        assert_eq!(guard_color_key(0x00010001), 0x00000001);
    }
}

impl Overlay {
    pub fn new(config: &Config) -> Self {
        OVERLAY_CONFIG.get_or_init(|| Arc::new(Mutex::new(config.clone())));
        update_config(config);

        unsafe {
            let hinstance = GetModuleHandleW(None).unwrap();
            let hinstance_win: windows::Win32::Foundation::HINSTANCE = hinstance.into();

            let wc = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance_win,
                lpszClassName: CLASS_NAME,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
                hbrBackground: HBRUSH(std::ptr::null_mut()),
                ..Default::default()
            };
            RegisterClassW(&wc);

            // Initial position on primary monitor (overlay starts hidden)
            let monitor = monitor_rect_for(HWND::default());
            let (x, y, w, h) = calc_window_rect(config, monitor);

            let ex_style = WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW;

            let hwnd = CreateWindowExW(
                ex_style,
                CLASS_NAME,
                w!("ClockOR"),
                WS_POPUP,
                x,
                y,
                w,
                h,
                None,
                None,
                hinstance_win,
                None,
            )
            .unwrap();

            let alpha = (config.opacity as f32 / 100.0 * 255.0) as u8;
            let _ = SetLayeredWindowAttributes(hwnd, COLOR_KEY, alpha, LWA_COLORKEY | LWA_ALPHA);

            SetTimer(hwnd, TIMER_ID, 1000, None);

            Overlay { hwnd }
        }
    }

    pub fn show(&self) {
        unsafe {
            let config = get_config();
            // Position on the foreground window's monitor (likely the game)
            let monitor = monitor_rect_for(GetForegroundWindow());
            let (x, y, w, h) = calc_window_rect(&config, monitor);
            let alpha = (config.opacity as f32 / 100.0 * 255.0) as u8;
            let _ =
                SetLayeredWindowAttributes(self.hwnd, COLOR_KEY, alpha, LWA_COLORKEY | LWA_ALPHA);
            let _ = SetWindowPos(self.hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn destroy(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
