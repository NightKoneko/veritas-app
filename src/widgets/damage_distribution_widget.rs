use eframe::egui::{Stroke, Ui};
use egui_plot::{Legend, Plot, PlotPoints, Polygon};

use crate::app::DamageAnalyzer;

impl DamageAnalyzer {
    pub fn show_damage_distribution_widget(&mut self, ui: &mut Ui) {
        Plot::new("damage_pie")
            .legend(Legend::default().position(egui_plot::Corner::RightTop))
            .height(300.0)
            .width(ui.available_width() * 0.5)
            .data_aspect(1.0)
            .clamp_grid(true)
            .show_grid(false)
            .show_background(false)
            .show_axes([false; 2])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                if let Some(buffer) = self.data_buffer.try_lock() {
                    let total: f64 = buffer.total_damage.values().sum::<f32>() as f64;
                    if total > 0.0 {
                        let segments =
                            Self::create_pie_segments(&buffer.total_damage, &buffer.column_names);

                        for (name, segment, i) in segments {
                            let color = self.get_character_color(i);
                            let percentage = segment.value / total * 100.0;

                            let plot_points = PlotPoints::new(segment.points);
                            let polygon = Polygon::new(plot_points)
                                .stroke(Stroke::new(1.5, color))
                                .name(format!(
                                    "{}: {:.1}% ({} dmg)",
                                    name,
                                    percentage,
                                    Self::format_damage(segment.value)
                                ));

                            plot_ui.polygon(polygon);
                        }
                    }
                }
            });
    }
}
