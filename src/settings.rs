use eframe::egui;

use crate::config::{Config, FontSize, Position, KEY_OPTIONS, MODIFIER_OPTIONS};

struct SettingsApp {
    config: Config,
    saved_config: Config,
    selected_mod: usize,
    selected_key: usize,
}

impl SettingsApp {
    fn new(config: Config) -> Self {
        let (mod_idx, key_idx) = Self::find_hotkey_indices(&config.hotkey);
        Self {
            saved_config: config.clone(),
            config,
            selected_mod: mod_idx,
            selected_key: key_idx,
        }
    }

    fn find_hotkey_indices(hotkey: &str) -> (usize, usize) {
        let parts: Vec<&str> = hotkey.split('+').map(str::trim).collect();
        let key_name = parts.last().unwrap_or(&"F12");
        let mod_str = if parts.len() >= 2 {
            parts[..parts.len() - 1].join("+")
        } else {
            "Ctrl".to_string()
        };

        let mod_idx = MODIFIER_OPTIONS
            .iter()
            .position(|(name, _)| name.eq_ignore_ascii_case(&mod_str))
            .unwrap_or(0);
        let key_idx = KEY_OPTIONS
            .iter()
            .position(|(name, _)| name.eq_ignore_ascii_case(key_name))
            .unwrap_or(KEY_OPTIONS.len() - 1);

        (mod_idx, key_idx)
    }

    fn build_hotkey_string(&self) -> String {
        let mod_name = MODIFIER_OPTIONS[self.selected_mod].0;
        let key_name = KEY_OPTIONS[self.selected_key].0;
        format!("{mod_name}+{key_name}")
    }

    fn current_config(&self) -> Config {
        let mut cfg = self.config.clone();
        cfg.hotkey = self.build_hotkey_string();
        cfg
    }

    fn has_unsaved_changes(&self) -> bool {
        self.current_config() != self.saved_config
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ClockOR Settings");
            ui.add_space(8.0);

            // Position
            ui.label("Position:");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.config.position, Position::TopLeft, "Top-Left");
                ui.radio_value(&mut self.config.position, Position::TopRight, "Top-Right");
                ui.radio_value(
                    &mut self.config.position,
                    Position::BottomLeft,
                    "Bottom-Left",
                );
                ui.radio_value(
                    &mut self.config.position,
                    Position::BottomRight,
                    "Bottom-Right",
                );
            });
            ui.add_space(4.0);

            // Format
            ui.label("Time Format:");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.config.format_24h, true, "24-hour");
                ui.radio_value(&mut self.config.format_24h, false, "12-hour");
            });
            ui.add_space(4.0);

            // Seconds
            ui.checkbox(&mut self.config.show_seconds, "Show seconds");
            ui.add_space(4.0);

            // Font size
            ui.label("Font Size:");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.config.font_size, FontSize::Small, "Small (16px)");
                ui.radio_value(
                    &mut self.config.font_size,
                    FontSize::Medium,
                    "Medium (22px)",
                );
                ui.radio_value(&mut self.config.font_size, FontSize::Large, "Large (30px)");
            });
            ui.add_space(4.0);

            // Opacity
            let mut opacity_f = self.config.opacity as f32;
            ui.add(
                egui::Slider::new(&mut opacity_f, 25.0..=100.0)
                    .text("Opacity %")
                    .integer(),
            );
            self.config.opacity = opacity_f as u8;
            ui.add_space(4.0);

            // Hotkey
            ui.label("Hotkey:");
            ui.horizontal(|ui| {
                let current_mod = MODIFIER_OPTIONS[self.selected_mod].0;
                egui::ComboBox::from_id_salt("modifier")
                    .selected_text(current_mod)
                    .show_ui(ui, |ui| {
                        for (i, (name, _)) in MODIFIER_OPTIONS.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_mod, i, *name);
                        }
                    });

                ui.label("+");

                let current_key = KEY_OPTIONS[self.selected_key].0;
                egui::ComboBox::from_id_salt("key")
                    .selected_text(current_key)
                    .show_ui(ui, |ui| {
                        for (i, (name, _)) in KEY_OPTIONS.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_key, i, *name);
                        }
                    });
            });
            ui.add_space(4.0);

            // Auto start
            ui.checkbox(&mut self.config.start_with_windows, "Start with Windows");
            ui.add_space(12.0);

            // Apply button + status
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() {
                    self.config.hotkey = self.build_hotkey_string();
                    if let Err(e) = self.config.save() {
                        eprintln!("Failed to save config: {e}");
                    }
                    crate::overlay::update_config(&self.config);
                    crate::apply_autostart(&self.config);
                    crate::request_hotkey_reregister();
                    self.saved_config = self.config.clone();
                }
                if !self.has_unsaved_changes() {
                    ui.label("Settings saved!");
                }
            });
        });
    }
}

pub fn open_settings(config: Config) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 380.0])
            .with_resizable(false)
            .with_always_on_top(),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "ClockOR Settings",
        options,
        Box::new(|_cc| Ok(Box::new(SettingsApp::new(config)))),
    );
}
