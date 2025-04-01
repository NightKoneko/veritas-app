use eframe::egui::Ui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::{app::DamageAnalyzer, core::helpers};

impl DamageAnalyzer {
    pub fn show_turn_damage_plot_widget(&mut self, ui: &mut Ui) {
        let data_buffer = self.data_buffer.blocking_lock().clone();
        Plot::new("damage_plot")
            .legend(Legend::default())
            .height(250.0)
            .include_y(0.0)
            .x_axis_label("Turn")
            .y_axis_label("Damage")
            .y_axis_formatter(|y, _| helpers::format_damage(y.value))
            .show(ui, |plot_ui| {
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
                let is_there_update = self.is_there_update.blocking_lock().clone();
                if is_there_update {
                    plot_ui.set_auto_bounds([true, true]);
                }
        });
    }
}
