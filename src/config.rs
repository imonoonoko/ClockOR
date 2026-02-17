use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::PathBuf;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_SHIFT, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2, VK_F3, VK_F4, VK_F5,
    VK_F6, VK_F7, VK_F8, VK_F9,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

/// Deserialize font_size from either a u32 or a legacy string ("small"/"medium"/"large").
fn deserialize_font_size<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum FontSizeValue {
        Number(u32),
        LegacyString(String),
    }

    match FontSizeValue::deserialize(deserializer)? {
        FontSizeValue::Number(n) => Ok(n),
        FontSizeValue::LegacyString(s) => match s.as_str() {
            "small" => Ok(16),
            "medium" => Ok(22),
            "large" => Ok(30),
            other => Err(de::Error::custom(format!("unknown font size: {other}"))),
        },
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextStyle {
    None,
    #[default]
    Outline,
    Shadow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub position: Position,
    pub format_24h: bool,
    pub show_seconds: bool,
    #[serde(deserialize_with = "deserialize_font_size")]
    pub font_size: u32,
    pub opacity: u8,
    pub hotkey: String,
    pub start_with_windows: bool,
    pub text_style: TextStyle,
    #[serde(default = "default_text_color")]
    pub text_color: [u8; 3],
    #[serde(default = "default_outline_color")]
    pub outline_color: [u8; 3],
}

fn default_text_color() -> [u8; 3] {
    [255, 255, 255]
}

fn default_outline_color() -> [u8; 3] {
    [0, 0, 0]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            position: Position::TopRight,
            format_24h: true,
            show_seconds: false,
            font_size: 22,
            opacity: 80,
            hotkey: "Ctrl+F12".to_string(),
            start_with_windows: false,
            text_style: TextStyle::default(),
            text_color: default_text_color(),
            outline_color: default_outline_color(),
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
    /// Convert text_color [R,G,B] to Win32 COLORREF (0x00BBGGRR)
    pub fn text_colorref(&self) -> u32 {
        self.text_color[0] as u32
            | ((self.text_color[1] as u32) << 8)
            | ((self.text_color[2] as u32) << 16)
    }

    /// Convert outline_color [R,G,B] to Win32 COLORREF (0x00BBGGRR)
    pub fn outline_colorref(&self) -> u32 {
        self.outline_color[0] as u32
            | ((self.outline_color[1] as u32) << 8)
            | ((self.outline_color[2] as u32) << 16)
    }

    pub fn parsed_hotkey(&self) -> (u32, u32) {
        parse_hotkey(&self.hotkey).unwrap_or((MOD_CONTROL.0, VK_F12.0 as u32))
    }

    pub fn load() -> Self {
        Self::load_from(&config_path())
    }

    pub fn load_from(path: &std::path::Path) -> Self {
        let mut config = if let Ok(content) = fs::read_to_string(path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            Config::default()
        };
        config.opacity = config.opacity.clamp(25, 100);
        config.font_size = config.font_size.clamp(10, 60);
        config
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.save_to(&config_path())
    }

    pub fn save_to(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_hotkey ---

    #[test]
    fn parse_hotkey_ctrl_f12() {
        let (m, k) = parse_hotkey("Ctrl+F12").unwrap();
        assert_eq!(m, MOD_CONTROL.0);
        assert_eq!(k, VK_F12.0 as u32);
    }

    #[test]
    fn parse_hotkey_alt_f1() {
        let (m, k) = parse_hotkey("Alt+F1").unwrap();
        assert_eq!(m, MOD_ALT.0);
        assert_eq!(k, VK_F1.0 as u32);
    }

    #[test]
    fn parse_hotkey_ctrl_shift_f5() {
        let (m, k) = parse_hotkey("Ctrl+Shift+F5").unwrap();
        assert_eq!(m, MOD_CONTROL.0 | MOD_SHIFT.0);
        assert_eq!(k, VK_F5.0 as u32);
    }

    #[test]
    fn parse_hotkey_case_insensitive() {
        let (m, k) = parse_hotkey("ctrl+f12").unwrap();
        assert_eq!(m, MOD_CONTROL.0);
        assert_eq!(k, VK_F12.0 as u32);
    }

    #[test]
    fn parse_hotkey_no_modifier() {
        assert!(parse_hotkey("F12").is_none());
    }

    #[test]
    fn parse_hotkey_empty() {
        assert!(parse_hotkey("").is_none());
    }

    #[test]
    fn parse_hotkey_unknown_key() {
        assert!(parse_hotkey("Ctrl+Z").is_none());
    }

    // --- Config::default ---

    #[test]
    fn default_config_values() {
        let cfg = Config::default();
        assert_eq!(cfg.position, Position::TopRight);
        assert!(cfg.format_24h);
        assert!(!cfg.show_seconds);
        assert_eq!(cfg.font_size, 22);
        assert_eq!(cfg.opacity, 80);
        assert_eq!(cfg.hotkey, "Ctrl+F12");
        assert!(!cfg.start_with_windows);
        assert_eq!(cfg.text_style, TextStyle::Outline);
        assert_eq!(cfg.text_color, [255, 255, 255]);
        assert_eq!(cfg.outline_color, [0, 0, 0]);
    }

    // --- color fields ---

    #[test]
    fn default_colors() {
        let cfg = Config::default();
        assert_eq!(cfg.text_color, [255, 255, 255]);
        assert_eq!(cfg.outline_color, [0, 0, 0]);
    }

    #[test]
    fn text_colorref_conversion() {
        let mut cfg = Config::default();
        // White: RGB(255,255,255) -> COLORREF 0x00FFFFFF
        assert_eq!(cfg.text_colorref(), 0x00FFFFFF);
        // Black: RGB(0,0,0) -> COLORREF 0x00000000
        assert_eq!(cfg.outline_colorref(), 0x00000000);
        // Red: RGB(255,0,0) -> COLORREF 0x000000FF
        cfg.text_color = [255, 0, 0];
        assert_eq!(cfg.text_colorref(), 0x000000FF);
        // Blue: RGB(0,0,255) -> COLORREF 0x00FF0000
        cfg.text_color = [0, 0, 255];
        assert_eq!(cfg.text_colorref(), 0x00FF0000);
    }

    #[test]
    fn color_roundtrip() {
        let dir = std::env::temp_dir().join("clockor_test_color_rt");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("config.toml");

        let mut cfg = Config::default();
        cfg.text_color = [128, 64, 32];
        cfg.outline_color = [10, 20, 30];
        cfg.save_to(&path).unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(loaded.text_color, [128, 64, 32]);
        assert_eq!(loaded.outline_color, [10, 20, 30]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_colors_default() {
        let dir = std::env::temp_dir().join("clockor_test_no_colors");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        // Old config without color fields
        fs::write(&path, "position = \"top-right\"\nopacity = 80\n").unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(loaded.text_color, [255, 255, 255]);
        assert_eq!(loaded.outline_color, [0, 0, 0]);
        let _ = fs::remove_dir_all(&dir);
    }

    // --- parsed_hotkey fallback ---

    #[test]
    fn parsed_hotkey_invalid_falls_back() {
        let mut cfg = Config::default();
        cfg.hotkey = "garbage".to_string();
        let (m, k) = cfg.parsed_hotkey();
        assert_eq!(m, MOD_CONTROL.0);
        assert_eq!(k, VK_F12.0 as u32);
    }

    // --- legacy font_size string deserialization ---

    #[test]
    fn legacy_font_size_small() {
        let dir = std::env::temp_dir().join("clockor_test_legacy_small");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "font_size = \"small\"\n").unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(loaded.font_size, 16);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_font_size_medium() {
        let dir = std::env::temp_dir().join("clockor_test_legacy_medium");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "font_size = \"medium\"\n").unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(loaded.font_size, 22);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_font_size_large() {
        let dir = std::env::temp_dir().join("clockor_test_legacy_large");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "font_size = \"large\"\n").unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(loaded.font_size, 30);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn font_size_numeric() {
        let dir = std::env::temp_dir().join("clockor_test_fs_numeric");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "font_size = 40\n").unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(loaded.font_size, 40);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn font_size_clamped() {
        let dir = std::env::temp_dir().join("clockor_test_fs_clamp");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "font_size = 100\n").unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(loaded.font_size, 60);
        let _ = fs::remove_dir_all(&dir);
    }

    // --- TextStyle round-trip ---

    #[test]
    fn text_style_roundtrip() {
        let dir = std::env::temp_dir().join("clockor_test_textstyle");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("config.toml");

        for style in [TextStyle::None, TextStyle::Outline, TextStyle::Shadow] {
            let mut cfg = Config::default();
            cfg.text_style = style;
            cfg.save_to(&path).unwrap();
            let loaded = Config::load_from(&path);
            assert_eq!(loaded.text_style, style);
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_style_defaults_when_missing() {
        let dir = std::env::temp_dir().join("clockor_test_ts_default");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "position = \"top-right\"\n").unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(loaded.text_style, TextStyle::Outline);
        let _ = fs::remove_dir_all(&dir);
    }

    // --- save/load round-trip ---

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir().join("clockor_test_roundtrip");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("config.toml");

        let mut cfg = Config::default();
        cfg.position = Position::BottomLeft;
        cfg.opacity = 50;
        cfg.show_seconds = true;
        cfg.hotkey = "Alt+F1".to_string();

        cfg.save_to(&path).unwrap();
        let loaded = Config::load_from(&path);

        assert_eq!(loaded.position, Position::BottomLeft);
        assert_eq!(loaded.opacity, 50);
        assert!(loaded.show_seconds);
        assert_eq!(loaded.hotkey, "Alt+F1");

        let _ = fs::remove_dir_all(&dir);
    }

    // --- invalid TOML → default ---

    #[test]
    fn invalid_toml_returns_default() {
        let dir = std::env::temp_dir().join("clockor_test_invalid");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "{{{{not valid toml!!!!").unwrap();

        let loaded = Config::load_from(&path);
        assert_eq!(loaded, Config::default());

        let _ = fs::remove_dir_all(&dir);
    }

    // --- opacity clamp ---

    #[test]
    fn opacity_clamped_to_25_minimum() {
        let dir = std::env::temp_dir().join("clockor_test_clamp");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "opacity = 5\n").unwrap();

        let loaded = Config::load_from(&path);
        assert_eq!(loaded.opacity, 25);

        let _ = fs::remove_dir_all(&dir);
    }

    // --- partial TOML → missing fields use defaults ---

    #[test]
    fn partial_toml_fills_defaults() {
        let dir = std::env::temp_dir().join("clockor_test_partial");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "position = \"bottom-right\"\n").unwrap();

        let loaded = Config::load_from(&path);
        assert_eq!(loaded.position, Position::BottomRight);
        // All other fields should be default
        assert!(loaded.format_24h);
        assert!(!loaded.show_seconds);
        assert_eq!(loaded.font_size, 22);
        assert_eq!(loaded.opacity, 80);
        assert_eq!(loaded.hotkey, "Ctrl+F12");

        let _ = fs::remove_dir_all(&dir);
    }
}
