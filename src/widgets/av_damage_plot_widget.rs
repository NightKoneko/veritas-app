use eframe::egui::Ui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::{app::DamageAnalyzer, core::helpers};

impl DamageAnalyzer {
    pub fn show_av_damage_plot_widget(&mut self, ui: &mut Ui) {
        Plot::new("dmg_av_plot")
            .legend(Legend::default())
            .height(250.0)
            .include_y(0.0)
            .x_axis_label("Action Value")
            .y_axis_label("Damage")
            .y_axis_formatter(|y, _| helpers::format_damage(y.value))
            .show(ui, |plot_ui| {
                let data_buffer = self.data_buffer.blocking_lock().clone();
                for (i, name) in data_buffer.column_names.iter().enumerate() {
                    let color = helpers::get_character_color(i);

                    let av_damages = data_buffer.av_damage
                        .iter()
                        .map(|dmg_map| dmg_map.get(name).unwrap())
                        .copied()
                        .collect::<Vec<f32>>();

                    let points = data_buffer.av_history
                        .iter()
                        .zip(av_damages.iter())
                        .map(|(x, y)| [*x as f64, *y as f64])
                        .collect::<Vec<[f64; 2]>>();

                    plot_ui.line(
                        Line::new(PlotPoints::new(points))
                            .name(name)
                            .color(color)
                            .width(2.0),
                    );
                }
                let is_there_update = self.is_there_update.blocking_lock().clone();
                if is_there_update {
                    plot_ui.set_auto_bounds([true, true]);
                }
            });
    }
}