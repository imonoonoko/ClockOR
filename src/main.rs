#![windows_subsystem = "windows"]

mod config;
mod overlay;
mod settings;

use config::Config;
use overlay::Overlay;

use std::sync::atomic::{AtomicBool, Ordering};

use muda::{Menu, MenuEvent, MenuItem};
#[allow(unused_imports)]
use tray_icon::{Icon, TrayIconBuilder};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MessageBoxW, MsgWaitForMultipleObjects, PeekMessageW, TranslateMessage,
    MB_ICONWARNING, MB_OK, MSG, PM_REMOVE, QS_ALLINPUT, WM_HOTKEY, WM_QUIT,
};

const HOTKEY_ID: i32 = 1;

static OVERLAY_VISIBLE: AtomicBool = AtomicBool::new(false);
static HOTKEY_REREGISTER: AtomicBool = AtomicBool::new(false);

pub fn request_hotkey_reregister() {
    HOTKEY_REREGISTER.store(true, Ordering::Relaxed);
}

fn register_hotkey(config: &Config) -> bool {
    let (modifiers, vk) = config.parsed_hotkey();
    unsafe { RegisterHotKey(HWND::default(), HOTKEY_ID, HOT_KEY_MODIFIERS(modifiers), vk).is_ok() }
}

fn unregister_hotkey() {
    unsafe {
        let _ = UnregisterHotKey(HWND::default(), HOTKEY_ID);
    }
}

fn show_hotkey_error(hotkey: &str) {
    let msg: Vec<u16> = format!(
        "Failed to register hotkey: {hotkey}\n\
         Another application may already be using this key combination."
    )
    .encode_utf16()
    .chain(std::iter::once(0))
    .collect();
    let title: Vec<u16> = "ClockOR".encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = MessageBoxW(
            HWND::default(),
            windows::core::PCWSTR(msg.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONWARNING,
        );
    }
}

fn create_default_icon() -> Icon {
    let size = 16u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let center = (size / 2) as f32;
    let radius = center - 1.0;

    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= radius {
                rgba[idx] = 100;
                rgba[idx + 1] = 180;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = 255;
            }
        }
    }
    for dy in 0..4 {
        let y = (center as u32) - dy;
        let x = center as u32;
        let idx = ((y * size + x) * 4) as usize;
        rgba[idx] = 255;
        rgba[idx + 1] = 255;
        rgba[idx + 2] = 255;
        rgba[idx + 3] = 255;
    }
    for dx in 0..5 {
        let y = center as u32;
        let x = (center as u32) + dx;
        let idx = ((y * size + x) * 4) as usize;
        rgba[idx] = 255;
        rgba[idx + 1] = 255;
        rgba[idx + 2] = 255;
        rgba[idx + 3] = 255;
    }

    Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}

pub fn apply_autostart(config: &Config) {
    use std::env;
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_WRITE,
        REG_SZ,
    };

    let key_path = HSTRING::from("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run");
    let value_name = HSTRING::from("ClockOR");

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let result = RegOpenKeyExW(HKEY_CURRENT_USER, &key_path, 0, KEY_WRITE, &mut hkey);
        if result.is_err() {
            return;
        }

        if config.start_with_windows {
            if let Ok(exe_path) = env::current_exe() {
                let path_str = exe_path.to_string_lossy();
                let wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
                let byte_len = wide.len() * std::mem::size_of::<u16>();
                let bytes = std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), byte_len);
                let _ = RegSetValueExW(hkey, &value_name, 0, REG_SZ, Some(bytes));
            }
        } else {
            let _ = RegDeleteValueW(hkey, &value_name);
        }

        let _ = RegCloseKey(hkey);
    }
}

fn main() {
    let config = Config::load();

    // Create overlay (hidden initially)
    let overlay = Overlay::new(&config);

    // Register hotkey from config
    if !register_hotkey(&config) {
        show_hotkey_error(&config.hotkey);
    }

    // Build tray menu
    let menu = Menu::new();
    let item_settings = MenuItem::new("Settings", true, None);
    let item_quit = MenuItem::new("Quit", true, None);
    let _ = menu.append(&item_settings);
    let _ = menu.append(&item_quit);

    let settings_id = item_settings.id().clone();
    let quit_id = item_quit.id().clone();

    // Build tray icon
    let icon = create_default_icon();
    let _tray = TrayIconBuilder::new()
        .with_tooltip("ClockOR - Press hotkey to toggle")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .expect("Failed to create tray icon");

    // Message loop
    let mut msg = MSG::default();
    'main_loop: loop {
        // Check if hotkey needs re-registration (from settings thread)
        if HOTKEY_REREGISTER.swap(false, Ordering::Relaxed) {
            unregister_hotkey();
            let fresh = Config::load();
            if !register_hotkey(&fresh) {
                show_hotkey_error(&fresh.hotkey);
            }
        }

        // Drain tray menu events
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == settings_id {
                // eframe/winit requires main thread on Windows — blocks until window closes
                let cfg = Config::load();
                settings::open_settings(cfg);
                // After settings closed, apply any hotkey changes
                if HOTKEY_REREGISTER.swap(false, Ordering::Relaxed) {
                    unregister_hotkey();
                    let fresh = Config::load();
                    if !register_hotkey(&fresh) {
                        show_hotkey_error(&fresh.hotkey);
                    }
                }
            } else if event.id == quit_id {
                overlay.destroy();
                break 'main_loop;
            }
        }

        // Process Win32 messages
        unsafe {
            while PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    break 'main_loop;
                }

                if msg.message == WM_HOTKEY && msg.wParam.0 == HOTKEY_ID as usize {
                    let was_visible = OVERLAY_VISIBLE.load(Ordering::Relaxed);
                    if was_visible {
                        overlay.hide();
                        OVERLAY_VISIBLE.store(false, Ordering::Relaxed);
                    } else {
                        let fresh = Config::load();
                        overlay::update_config(&fresh);
                        overlay.show();
                        OVERLAY_VISIBLE.store(true, Ordering::Relaxed);
                    }
                }

                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Wait for messages or 50ms timeout (zero CPU when idle,
            // wakes immediately on Win32 message, checks atomic flags every 50ms)
            MsgWaitForMultipleObjects(None, false, 50, QS_ALLINPUT);
        }
    }

    unregister_hotkey();
}
