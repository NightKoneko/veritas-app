mod app;
mod models;
mod network;

use eframe::egui;

#[tokio::main]
async fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_window_level(egui::WindowLevel::Normal),
        ..Default::default()
    };

    eframe::run_native(
        "Veritas App",
        options,
        Box::new(|cc| Box::new(app::DamageAnalyzer::new(cc))),
    )
}
