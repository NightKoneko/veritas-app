use eframe::egui::{self, Button, Color32};

use crate::app::DamageAnalyzer;

impl DamageAnalyzer {
    pub fn show_sidebar_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("Sidebar")
            .max_width(48.0)
            .resizable(false)
            .show_separator_line(!self.is_sidebar_expanded)
            .show(ctx, |ui| {
                ui.with_layout(
                    egui::Layout::from_main_dir_and_cross_align(
                        egui::Direction::TopDown,
                        egui::Align::Center,
                    ),
                    |ui| {
                        let menu_icon = egui_material_icons::icon_text(egui_material_icons::icons::ICON_MENU);
                        let button = Button::new(menu_icon.size(20.0))
                            .fill(Color32::TRANSPARENT)
                            .frame(false);

                        if ui.add_sized([23.0, 23.0], button).clicked() {
                            self.is_sidebar_expanded = !self.is_sidebar_expanded;
                        }
                    },
                );
            });

        egui::SidePanel::left("Sidebar_MainPanel")
            .resizable(false) // make the side panel resizable
            .min_width(100.0) // Minimum width of the side panel
            .show_animated(ctx, self.is_sidebar_expanded, |ui| {
                // TODO: add more buttons
                self.show_log_widget(ui)
            });
    }
}
