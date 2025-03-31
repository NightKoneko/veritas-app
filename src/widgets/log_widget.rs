use eframe::egui::{self, Ui};

use crate::app::DamageAnalyzer;

impl DamageAnalyzer {
    pub fn show_log_widget(&mut self, ui: &mut Ui) {
        ui.heading("Logs");
        let message_logger = self.message_logger.blocking_lock().clone();
        let text = message_logger.get_text();
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .max_height(ui.available_height() - 10.0)
            .show(ui, |ui| {
                let _response = ui.add(
                    egui::TextEdit::multiline(&mut text.as_str())
                        .desired_width(f32::INFINITY)
                        .desired_rows(1)
                        .frame(false)
                        .margin(egui::vec2(2.0, 2.0)),
                );
            });
    }
}