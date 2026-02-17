use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_SHIFT, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8,
    VK_F9, VK_F10, VK_F11, VK_F12,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontSize {
    Small,
    Medium,
    Large,
}

impl FontSize {
    pub fn pixel_size(self) -> i32 {
        match self {
            FontSize::Small => 16,
            FontSize::Medium => 22,
            FontSize::Large => 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub position: Position,
    pub format_24h: bool,
    pub show_seconds: bool,
    pub font_size: FontSize,
    pub opacity: u8,
    pub hotkey: String,
    pub start_with_windows: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            position: Position::TopRight,
            format_24h: true,
            show_seconds: false,
            font_size: FontSize::Medium,
            opacity: 80,
            hotkey: "Ctrl+F12".to_string(),
            start_with_windows: false,
        }
    }
}

fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("ClockOR");
    path.push("config.toml");
    path
}

pub const MODIFIER_OPTIONS: &[(&str, u32)] = &[
    ("Ctrl", MOD_CONTROL.0),
    ("Alt", MOD_ALT.0),
    ("Shift", MOD_SHIFT.0),
    ("Ctrl+Alt", MOD_CONTROL.0 | MOD_ALT.0),
    ("Ctrl+Shift", MOD_CONTROL.0 | MOD_SHIFT.0),
    ("Alt+Shift", MOD_ALT.0 | MOD_SHIFT.0),
];

pub const KEY_OPTIONS: &[(&str, u32)] = &[
    ("F1", VK_F1.0 as u32),
    ("F2", VK_F2.0 as u32),
    ("F3", VK_F3.0 as u32),
    ("F4", VK_F4.0 as u32),
    ("F5", VK_F5.0 as u32),
    ("F6", VK_F6.0 as u32),
    ("F7", VK_F7.0 as u32),
    ("F8", VK_F8.0 as u32),
    ("F9", VK_F9.0 as u32),
    ("F10", VK_F10.0 as u32),
    ("F11", VK_F11.0 as u32),
    ("F12", VK_F12.0 as u32),
];

/// Parse hotkey string like "Ctrl+F12" into (modifiers, vk_code).
pub fn parse_hotkey(hotkey: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = hotkey.split('+').map(str::trim).collect();
    if parts.len() < 2 {
        return None;
    }

    let key_name = parts.last()?;
    let vk = KEY_OPTIONS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key_name))?
        .1;

    let mod_str = parts[..parts.len() - 1].join("+");
    let modifiers = MODIFIER_OPTIONS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(&mod_str))?
        .1;

    Some((modifiers, vk))
}

impl Config {
    pub fn parsed_hotkey(&self) -> (u32, u32) {
        parse_hotkey(&self.hotkey).unwrap_or((MOD_CONTROL.0, VK_F12.0 as u32))
    }

    pub fn load() -> Self {
        let path = config_path();
        let mut config = if let Ok(content) = fs::read_to_string(&path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            let config = Config::default();
            let _ = config.save();
            config
        };
        config.opacity = config.opacity.clamp(25, 100);
        config
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
