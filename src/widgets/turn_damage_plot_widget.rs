use eframe::egui::Ui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use tokio::sync::MutexGuard;

use crate::{app::DamageAnalyzer, core::{helpers, models::DataBufferInner}};

impl DamageAnalyzer {
    pub fn show_turn_damage_plot_widget(&mut self, ui: &mut Ui) {
        Plot::new("damage_plot")
            .legend(Legend::default())
            .height(250.0)
            .include_y(0.0)
            .x_axis_label("Turn")
            .y_axis_label("Damage")
            .y_axis_formatter(|y, _| helpers::format_damage(y.value))
            .show(ui, |plot_ui| {
                if let Ok(data_buffer) = self.data_buffer.try_lock() {
                    for (i, name) in data_buffer.column_names.iter().enumerate() {
                        let color = helpers::get_character_color(i);
                        let damage_points = &data_buffer.turn_damage
                            .iter()
                            .enumerate()
                            .map(|(i, dmg_map)| {
                                [(i + 1) as f64, *dmg_map.get(name).unwrap() as f64]
                            })
                            .collect::<Vec<[f64; 2]>>();
                        
                        if !damage_points.is_empty() {
                            plot_ui.line(
                                Line::new(PlotPoints::from(damage_points.clone()))
                                    .name(name)
                                    .color(color)
                                    .width(2.0),
                            );
                        }
                    }
                    if let Ok(is_there_update) = self.is_there_update.try_lock() {
                        if *is_there_update {
                            plot_ui.set_auto_bounds([true, true]);
                        }
                    }
                }
        });
    }
}
