use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use crate::app::DamageAnalyzer;

impl DamageAnalyzer {
    pub fn show_av_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::right("av_panel")
            .resizable(true)
            .default_width(250.0)
            .width_range(200.0..=400.0)
            .show(ctx, |ui| {
                ui.heading("Action Value Metrics");

                if let Some(buffer) = self.data_buffer.try_lock() {
                    ui.separator();
                    ui.label("Current Turn");
                    ui.horizontal(|ui| {
                        ui.label("AV:");
                        ui.label(format!("{:.2}", buffer.current_av));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Total Damage:");
                        ui.label(Self::format_damage(
                            buffer.total_damage.values().sum::<f32>() as f64,
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Total DpAV:");
                        ui.label(format!("{:.2}", buffer.total_dpav));
                    });

                    ui.separator();
                    ui.label("DpAV over Time");
                    Plot::new("dpav_plot")
                        .height(200.0)
                        .include_y(0.0)
                        .auto_bounds_x()
                        .allow_drag(false)
                        .allow_zoom(false)
                        .x_axis_label("Turn")
                        .y_axis_label("DpAV")
                        .y_axis_formatter(|y, _| format!("{:.1}", y.value))
                        .show(ui, |plot_ui| {
                            if !buffer.dpav_history.is_empty() {
                                let points: Vec<[f64; 2]> = buffer
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
                            }
                        });

                    ui.separator();
                    // this is kind of scuffed I think
                    ui.label("Damage vs Action Value");
                    Plot::new("dmg_av_plot")
                        .height(200.0)
                        .include_y(0.0)
                        .auto_bounds_x()
                        .allow_drag(false)
                        .allow_zoom(false)
                        .x_axis_label("Action Value")
                        .y_axis_label("Damage")
                        .y_axis_formatter(|y, _| Self::format_damage(y.value))
                        .show(ui, |plot_ui| {
                            if !buffer.av_history.is_empty() {
                                for (i, name) in buffer.column_names.iter().enumerate() {
                                    let color = self.get_character_color(i);
                                    let mut points: Vec<(f32, f32)> = buffer.av_history.iter()
                                        .zip(0..buffer.turn_damage.len())
                                        .map(|(&av, turn_idx)| {
                                            let damage = buffer.turn_damage
                                                .get(turn_idx)
                                                .and_then(|turn| turn.get(name))
                                                .copied()
                                                .unwrap_or(0.0);
                                            (av, damage)
                                        })
                                        .collect();

                                    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

                                    let plot_points = points.into_iter()
                                        .map(|(av, damage)| [av as f64, damage as f64])
                                        .collect::<Vec<_>>();

                                    plot_ui.line(
                                        Line::new(PlotPoints::from(plot_points))
                                            .name(name)
                                            .color(color)
                                            .width(2.0),
                                    );
                                }
                            }
                        });
                }
            });
    }
}
