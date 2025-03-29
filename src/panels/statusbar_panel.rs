use eframe::egui;

use crate::app::DamageAnalyzer;

impl DamageAnalyzer {
    fn toggle_pin(&mut self) {
        self.window_pinned = !self.window_pinned;
        let mut message_logger = self.message_logger.blocking_lock();
        message_logger.log_message(if self.window_pinned {
            "Window pinned on top"
        } else {
            "Window unpinned"
        });
    }

    pub fn show_statusbar_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let binding = self.connected.clone();
                let connected = binding.blocking_lock();
                ui.label(if *connected {
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
