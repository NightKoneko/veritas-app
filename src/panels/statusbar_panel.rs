use eframe::{egui::{self, Align, Button, Color32, CornerRadius, Label, Shadow, Vec2}, Frame};
use egui_material_icons::icons::ICON_WIFI;

use crate::app::DamageAnalyzer;

impl DamageAnalyzer {
    fn toggle_pin(&mut self) {
        self.state.is_window_pinned = !self.state.is_window_pinned;
        let mut message_logger = self.message_logger.blocking_lock();
        message_logger.log(if self.state.is_window_pinned {
            "Window pinned on top"
        } else {
            "Window unpinned"
        });    
    }

    pub fn show_statusbar_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // System Color Statusbar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().interaction.selectable_labels = false;

                let connected = self.connected.blocking_lock().clone();

                if connected {
                    ui.add(Label::new(egui_material_icons::icon_text(ICON_WIFI).color(egui::Color32::from_rgb(0, 180, 0))));
                } else {
                    ui.add(Label::new(egui_material_icons::icon_text(ICON_WIFI).color(egui::Color32::from_rgb(255, 180, 0))));
                }

                ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                    ui.label(if connected {
                        egui::RichText::new("Connected").color(egui::Color32::from_rgb(0, 180, 0))
                    } else {
                        // TODO: Make this not look terrible on light mode
                        egui::RichText::new("Connecting...").color(egui::Color32::from_rgb(255, 180, 0))
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(if self.state.is_window_pinned {
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
