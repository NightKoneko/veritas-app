use eframe::egui;

use crate::app::DamageAnalyzer;

impl DamageAnalyzer {
    pub fn show_statusbar_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(if self.connected {
                    egui::RichText::new("Connected").color(egui::Color32::GREEN)
                } else {
                    // TODO: Make this not look terrible on light mode
                    egui::RichText::new("Connecting...").color(egui::Color32::YELLOW)
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(if self.window_pinned {
                            "Unpin Window"
                        } else {
                            "Pin Window"
                        })
                        .clicked()
                    {
                        self.toggle_pin();
                    }
                });
            });
        });
    }
}
