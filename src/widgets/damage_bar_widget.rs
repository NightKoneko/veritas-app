use eframe::egui::Ui;
use egui_plot::{Bar, BarChart, Legend, Plot};

use crate::app::DamageAnalyzer;
use crate::core::helpers;
use crate::core::models::DataBufferInner;

fn create_bar_data(buffer: &DataBufferInner) -> Vec<(String, f64, usize)> {        
    buffer.column_names.iter()
        .enumerate()
        .filter_map(|(i, name)| {
            buffer.total_damage.get(name)
                .map(|&damage| (name.clone(), damage as f64, i))
        })
        .collect()
}

impl DamageAnalyzer {
    pub fn show_damage_bar_widget(&mut self, ui: &mut Ui) {
        let data_buffer = self.data_buffer.blocking_lock().clone();
        Plot::new("damage_bars")
            .legend(Legend::default())
            .height(300.0)
            .width(ui.available_width())
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .y_axis_formatter(|y, _| helpers::format_damage(y.value))
            .x_axis_formatter(|x, _| {
                let bars_data = create_bar_data(&data_buffer);
                if let Some((name, _, _)) = bars_data.get(x.value.floor() as usize) {
                    return name.clone();
                }
                String::new()
            })
            .show(ui, |plot_ui| {
                let data_buffer = self.data_buffer.blocking_lock().clone();
                let bars_data = create_bar_data(&data_buffer);
                drop(data_buffer);
                let bars: Vec<Bar> = bars_data
                    .iter()
                    .enumerate()
                    .map(|(pos, (name, value, color_idx))| {
                        Bar::new(pos as f64, *value)
                            .name(name)
                            .fill(helpers::get_character_color(*color_idx))
                            .width(0.7)
                    })
                    .collect();

                let chart = BarChart::new(bars);
                plot_ui.bar_chart(chart);
        });
    }
}
