mod app;
mod core;
mod widgets;
mod panels;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_window_level(egui::WindowLevel::Normal),
        ..Default::default()
    };

    eframe::run_native(
        "Veritas",
        options,
        Box::new(|cc| Ok(Box::new(app::DamageAnalyzer::new(cc)))),
    )
}
