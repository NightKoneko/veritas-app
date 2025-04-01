use eframe::egui::{self, Ui};
use tokio::sync::MutexGuard;

use crate::{app::{DamageAnalyzer, Unit}, core::models::DataBufferInner};

impl DamageAnalyzer {
    pub fn show_central_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui: &mut Ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.group(|ui| {
                        ui.heading("Real-time Damage");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.state.graph_x_unit, Unit::Turn, "Turn");
                            ui.radio_value(&mut self.state.graph_x_unit, Unit::ActionValue, "Action Value");
                        });
                        match self.state.graph_x_unit {
                            Unit::Turn => self.show_turn_damage_plot_widget(ui),
                            Unit::ActionValue => self.show_av_damage_plot_widget(ui),
                        };
                    });

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.group(|ui| {
                                ui.heading("Damage Distribution");
                                self.show_damage_distribution_widget(ui);
                            });
                        });

                        ui.vertical(|ui| {
                            ui.group(|ui| {
                                ui.heading("Total Damage by Character");
                                self.show_damage_bar_widget(ui);
                            });
                        });
                    });
                });
            });
        });
    }
}
