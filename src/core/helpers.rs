use eframe::egui;

pub fn format_damage(value: f64) -> String {
    if value >= 1_000_000.0 {
        let m = value / 1_000_000.0;
        if m.fract() < 0.1 {
            format!("{}M", m.floor())
        } else {
            format!("{:.1}M", (value / 1_000_000.0).floor() * 10.0 / 10.0)
        }
    } else if value >= 1_000.0 {
        format!("{}K", (value / 1_000.0).floor())
    } else {
        format!("{}", value.floor())
    }
}

pub fn get_character_color(index: usize) -> egui::Color32 {
    const COLORS: &[egui::Color32] = &[
        egui::Color32::from_rgb(255, 99, 132),   
        egui::Color32::from_rgb(54, 162, 235),   
        egui::Color32::from_rgb(255, 206, 86),   
        egui::Color32::from_rgb(75, 192, 192),   
        egui::Color32::from_rgb(153, 102, 255),  
        egui::Color32::from_rgb(255, 159, 64),   
        egui::Color32::from_rgb(231, 233, 237),  
        egui::Color32::from_rgb(102, 255, 102),  
    ];
    
    COLORS[index % COLORS.len()]
}