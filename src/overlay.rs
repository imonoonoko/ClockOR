use std::sync::{Arc, Mutex};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetMonitorInfoW,
    InvalidateRect, MonitorFromWindow, SelectObject, SetBkMode, SetTextColor, TextOutW,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, FF_SWISS, FW_BOLD, HBRUSH, HGDIOBJ,
    MONITORINFO, MONITOR_DEFAULTTOPRIMARY, OUT_TT_PRECIS, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetForegroundWindow,
    GetSystemMetrics, KillTimer, LoadCursorW, PostQuitMessage, RegisterClassW,
    SetLayeredWindowAttributes, SetTimer, SetWindowPos, ShowWindow, HWND_TOPMOST, IDC_ARROW,
    LWA_ALPHA, LWA_COLORKEY, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SW_HIDE,
    SW_SHOWNOACTIVATE, WM_DESTROY, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::config::{Config, Position};

const TIMER_ID: usize = 1;
const CLASS_NAME: PCWSTR = w!("ClockOR_Overlay");
/// Color key for transparent background (RGB 1,0,1 — nearly black, won't match text)
const COLOR_KEY: COLORREF = COLORREF(0x00010001);

static OVERLAY_CONFIG: std::sync::OnceLock<Arc<Mutex<Config>>> = std::sync::OnceLock::new();

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
            (0, 0, GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
        }
    }
}

fn calc_window_rect(config: &Config, monitor: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    let (mon_x, mon_y, mon_w, mon_h) = monitor;
    let font_px = config.font_size.pixel_size();

    // Approximate character width: ~0.6 * font height for proportional font
    let char_w = (font_px as f32 * 0.6) as i32;
    let text_chars = match (config.format_24h, config.show_seconds) {
        (true, true) => 8,   // "HH:MM:SS"
        (true, false) => 5,  // "HH:MM"
        (false, true) => 11, // "HH:MM:SS AM"
        (false, false) => 8, // "HH:MM AM"
    };
    let text_w = char_w * text_chars;
    let win_w = text_w + 24;
    let win_h = font_px + 16;
    let margin = 10;

    let (x, y) = match config.position {
        Position::TopRight => (mon_x + mon_w - win_w - margin, mon_y + margin),
        Position::TopLeft => (mon_x + margin, mon_y + margin),
        Position::BottomRight => (mon_x + mon_w - win_w - margin, mon_y + mon_h - win_h - margin),
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

            // Text: white
            let font = CreateFontW(
                config.font_size.pixel_size(),
                0, 0, 0,
                FW_BOLD.0 as i32,
                0, 0, 0,
                DEFAULT_CHARSET.0 as u32,
                OUT_TT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                5, // CLEARTYPE_QUALITY
                (DEFAULT_PITCH.0 | FF_SWISS.0) as u32,
                w!("Segoe UI"),
            );
            let old_font = SelectObject(hdc, HGDIOBJ(font.0));
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, COLORREF(0x00FFFFFF));

            let time_str = format_time(&config);
            let wide: Vec<u16> = time_str.encode_utf16().collect();
            let _ = TextOutW(hdc, 12, 8, &wide);

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

            let ex_style = WS_EX_TOPMOST
                | WS_EX_TRANSPARENT
                | WS_EX_LAYERED
                | WS_EX_TOOLWINDOW;

            let hwnd = CreateWindowExW(
                ex_style,
                CLASS_NAME,
                w!("ClockOR"),
                WS_POPUP,
                x, y, w, h,
                None, None,
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
            let _ = SetLayeredWindowAttributes(self.hwnd, COLOR_KEY, alpha, LWA_COLORKEY | LWA_ALPHA);
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
