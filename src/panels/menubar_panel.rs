use eframe::egui::{self, ComboBox};

use crate::app::{DamageAnalyzer, Theme};

impl DamageAnalyzer {
    pub fn show_menubar_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Connection Settings...").clicked() {
                        self.state.show_connection_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("Preferences...").clicked() {
                        self.state.show_preferences = true;
                        ui.close_menu();
                    }
                });
            });
        });

        if self.state.show_connection_settings {
            egui::Window::new("Connection Settings")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Server:");
                        let binding = self.server_addr.clone();
                        let mut server_addr = binding.blocking_lock();
                        ui.text_edit_singleline(&mut (*server_addr));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Port:");
                        let binding = self.server_port.clone();
                        let mut server_port = binding.blocking_lock();
                        ui.text_edit_singleline(&mut (*server_port));
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.state.show_connection_settings = false;
                        }
                    });
                });
        }

        if self.state.show_preferences {
            egui::Window::new("Preferences")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        let mut selected_theme = self.state.theme;
                        ComboBox::from_id_salt("theme_selector")
                            .selected_text(self.state.theme.name())
                            .show_ui(ui, |ui| {
                                for &theme in Theme::ALL {
                                    let text = theme.name();
                                    if ui
                                        .selectable_value(&mut selected_theme, theme, text)
                                        .clicked()
                                    {
                                        self.set_theme(theme, ctx);
                                    }
                                }
                            });
                    });

                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.state.show_preferences = false;
                    }
                });
        }
    }
}
