use eframe::egui::Ui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::app::DamageAnalyzer;

impl DamageAnalyzer {
    pub fn show_turn_damage_plot_widget(&mut self, ui: &mut Ui) {
        Plot::new("damage_plot")
            .legend(Legend::default())
            .height(250.0)
            .include_y(0.0)
            .auto_bounds([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .x_axis_label("Turn")
            .y_axis_label("Damage")
            .y_axis_formatter(|y, _| Self::format_damage(y.value))
            .show(ui, |plot_ui| {
                if let Some(buffer) = self.data_buffer.try_lock() {
                    for (i, name) in buffer.column_names.iter().enumerate() {
                        let color = self.get_character_color(i);
                        let damage_points: Vec<[f64; 2]> = (0..buffer.turn_damage.len())
                            .map(|turn_idx| {
                                let damage = buffer
                                    .turn_damage
                                    .get(turn_idx)
                                    .and_then(|turn| turn.get(name))
                                    .copied()
                                    .unwrap_or(0.0);
                                [turn_idx as f64 + 1.0, damage as f64]
                            })
                            .collect();

                        if !damage_points.is_empty() {
                            plot_ui.line(
                                Line::new(PlotPoints::from(damage_points))
                                    .name(name)
                                    .color(color)
                                    .width(2.0),
                            );
                        }
                    }
                }
            });
    }
}
