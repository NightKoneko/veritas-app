use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use crate::{app::DamageAnalyzer, core::helpers};

impl DamageAnalyzer {
    pub fn show_av_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let data_buffer = self.data_buffer.blocking_lock().clone();
        egui::SidePanel::right("av_panel")
        .resizable(true)
        .default_width(250.0)
        .width_range(200.0..=400.0)
        .show(ctx, |ui| {
            ui.style_mut().interaction.selectable_labels = false;

            ui.heading("Action Value Metrics");

            ui.separator();
            ui.label("Current Turn");
            ui.horizontal(|ui| {
                
                ui.label("AV:");
                ui.label(format!("{:.2}", data_buffer.current_av));
            });
            ui.horizontal(|ui| {
                ui.label("Total Damage:");
                ui.label(helpers::format_damage(
                    data_buffer.total_damage.values().sum::<f32>() as f64
                ));
            });
            ui.horizontal(|ui| {
                ui.label("Total DpAV:");
                ui.label(format!("{:.2}", data_buffer.total_dpav));
            });

            ui.separator();
            ui.label("DpAV over Time");
            Plot::new("dpav_plot")
                .height(200.0)
                .include_y(0.0)
                .allow_drag(false)
                .allow_zoom(false)
                .x_axis_label("Turn")
                .y_axis_label("DpAV")
                .y_axis_formatter(|y, _| format!("{:.1}", y.value))
                .show(ui, |plot_ui| {
                        let points: Vec<[f64; 2]> = data_buffer
                            .dpav_history
                            .iter()
                            .enumerate()
                            .map(|(i, &dpav)| [i as f64 + 1.0, dpav as f64])
                            .collect();
                        plot_ui.line(
                            Line::new(PlotPoints::from(points))
                                .name("DpAV")
                                .width(2.0),
                        );
                });
        });
    }
}
