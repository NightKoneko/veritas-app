#![cfg_attr(
    all(
      target_os = "windows",
      not(debug_assertions),
    ),
    windows_subsystem = "windows"
)]
mod app;
mod core;
mod widgets;
mod panels;

use eframe::egui::{self, IconData};
use std::sync::Arc;


fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_window_level(egui::WindowLevel::Normal)
            .with_icon(load_icon().unwrap_or_else(|| Arc::new(IconData {
                rgba: Vec::new(),
                width: 0,
                height: 0,
            }))),
        ..Default::default()
    };

    eframe::run_native(
        "Veritas",
        options,
        Box::new(|cc| Ok(Box::new(app::DamageAnalyzer::new(cc)))),
    )
}

fn load_icon() -> Option<Arc<IconData>> {
    const ICON: &[u8] = include_bytes!("assets/veritas.ico");
    image::load_from_memory(ICON)
        .ok()
        .and_then(|img| {
            let rgba = img.into_rgba8();
            Some(Arc::new(IconData {
                rgba: rgba.to_vec(),
                width: rgba.width(),
                height: rgba.height(),
            }))
        })
}