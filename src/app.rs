use std::sync::Arc;
use std::fs::{self, File};
use std::collections::HashMap;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, Bar, BarChart, Legend};
use tokio::sync::mpsc;
use csv::Writer;
use serde::Deserialize;
use crate::{models::*, network::NetworkClient};

pub struct DamageAnalyzer {
    server_addr: String,
    server_port: String,
    network: NetworkClient,
    connected: bool,
    data_buffer: Arc<DataBuffer>,
    log_messages: Vec<String>,
    rx: Option<mpsc::Receiver<Packet>>,
    csv_writer: Option<Writer<File>>,
    current_file: String,
    window_pinned: bool,
    next_connect_attempt: Option<std::time::Instant>,
    show_connection_settings: bool,
}

impl DamageAnalyzer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        
        Self {
            server_addr: "127.0.0.1".to_string(),
            server_port: "1305".to_string(),
            network: NetworkClient::new(),
            connected: false,
            data_buffer: Arc::new(DataBuffer::new()),
            log_messages: Vec::new(),
            rx: None,
            csv_writer: None,
            current_file: String::new(),
            window_pinned: false,
            next_connect_attempt: None,
            show_connection_settings: false,
        }
    }
}

impl eframe::App for DamageAnalyzer {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Connection Settings...").clicked() {
                        self.show_connection_settings = true;
                        ui.close_menu();
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(if self.connected {
                    egui::RichText::new("Connected").color(egui::Color32::GREEN)
                } else {
                    egui::RichText::new("Connecting...").color(egui::Color32::YELLOW)
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if self.window_pinned { "Unpin Window" } else { "Pin Window" }).clicked() {
                        self.toggle_pin();
                    }
                });
            });
        });

        egui::SidePanel::left("log_panel")
            .resizable(true)
            .default_width(300.0)
            .width_range(200.0..=400.0)
            .show(ctx, |ui| {
                ui.heading("Logs");
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .max_height(ui.available_height() - 10.0)
                    .show(ui, |ui| {
                        for message in &self.log_messages {
                            ui.label(message);
                        }
                    });
            });

        egui::SidePanel::right("av_panel")
            .resizable(true)
            .default_width(250.0)
            .width_range(200.0..=400.0)
            .show(ctx, |ui| {
                ui.heading("Action Value Metrics");
                
                if let Some(buffer) = self.data_buffer.try_lock() {
                    ui.separator();
                    ui.label("Current Turn");
                    ui.horizontal(|ui| {
                        ui.label("AV:");
                        ui.label(format!("{:.2}", buffer.current_av));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Total Damage:");
                        ui.label(Self::format_damage(buffer.total_damage.values().sum::<f32>() as f64));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Total DpAV:");
                        ui.label(format!("{:.2}", buffer.total_dpav));
                    });

                    ui.separator();
                    ui.label("DpAV over Time");
                    Plot::new("dpav_plot")
                        .height(200.0)
                        .include_y(0.0)
                        .auto_bounds_y()
                        .allow_drag(false)
                        .allow_zoom(false)
                        .show(ui, |plot_ui| {
                            if !buffer.dpav_history.is_empty() {
                                let points: Vec<[f64; 2]> = buffer.dpav_history.iter()
                                    .enumerate()
                                    .map(|(i, &dpav)| [i as f64 + 1.0, dpav as f64])
                                    .collect();

                                plot_ui.line(Line::new(PlotPoints::from(points))
                                    .name("DpAV")
                                    .width(2.0));
                            }
                        });

                    ui.separator();
                    // this is kind of scuffed I think
                    ui.label("Damage vs Action Value");
                    Plot::new("dmg_av_plot")
                        .height(200.0)
                        .include_y(0.0)
                        .auto_bounds_y()
                        .allow_drag(false)
                        .allow_zoom(false)
                        .x_axis_label("Action Value")
                        .y_axis_label("Damage")
                        .y_axis_formatter(|y, _, _| Self::format_damage(y))
                        .show(ui, |plot_ui| {
                            if !buffer.turn_damage.is_empty() {
                                for (i, name) in buffer.column_names.iter().enumerate() {
                                    let color = self.get_character_color(i);
                                    let points: Vec<[f64; 2]> = (0..buffer.turn_damage.len())
                                        .map(|turn_idx| {
                                            let damage = buffer.turn_damage.get(turn_idx)
                                                .and_then(|turn| turn.get(name))
                                                .copied()
                                                .unwrap_or(0.0);
                                            let av = buffer.av_history.get(turn_idx)
                                                .copied()
                                                .unwrap_or(0.0);
                                            [av as f64, damage as f64]
                                        })
                                        .collect();

                                    if !points.is_empty() {
                                        plot_ui.line(Line::new(PlotPoints::from(points))
                                            .name(name)
                                            .color(color)
                                            .width(2.0));
                                    }
                                }
                            }
                        });
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.group(|ui| {
                        ui.heading("Real-time Damage");
                        Plot::new("damage_plot")
                            .legend(Legend::default())
                            .height(250.0)
                            .include_y(0.0)
                            .auto_bounds_y()
                            .allow_drag(false)
                            .allow_zoom(false)
                            .x_axis_label("Turn")
                            .y_axis_label("Damage")
                            .y_axis_formatter(|y, _, _| Self::format_damage(y))
                            .show(ui, |plot_ui| {
                                if let Some(buffer) = self.data_buffer.try_lock() {
                                    for (i, name) in buffer.column_names.iter().enumerate() {
                                        let color = self.get_character_color(i);
                                        let damage_points: Vec<[f64; 2]> = (0..buffer.turn_damage.len())
                                            .map(|turn_idx| {
                                                let damage = buffer.turn_damage.get(turn_idx)
                                                    .and_then(|turn| turn.get(name))
                                                    .copied()
                                                    .unwrap_or(0.0);
                                                [turn_idx as f64 + 1.0, damage as f64]
                                            })
                                            .collect();
        
                                        if !damage_points.is_empty() {
                                            plot_ui.line(Line::new(PlotPoints::from(damage_points))
                                                .name(name)
                                                .color(color)
                                                .width(2.0));
                                        }
                                    }
                                }
                            });
                    });
        
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.group(|ui| {
                                ui.heading("Damage Distribution");
                                Plot::new("damage_pie")
                                    .legend(Legend::default().position(egui_plot::Corner::RightTop))
                                    .height(300.0)
                                    .width(ui.available_width() * 0.5)
                                    .data_aspect(1.0)
                                    .allow_drag(false)
                                    .allow_zoom(false)
                                    .show(ui, |plot_ui| {
                                        if let Some(buffer) = self.data_buffer.try_lock() {
                                            let total: f64 = buffer.total_damage.values().sum::<f32>() as f64;
                                            if total > 0.0 {
                                                let segments = Self::create_pie_segments(&buffer.total_damage, &buffer.column_names);
                                                for (name, segment, i) in segments {
                                                    let color = self.get_character_color(i);
                                                    let percentage = segment.value / total * 100.0;
                                                    
                                                    plot_ui.line(Line::new(PlotPoints::from_iter(segment.points.iter().copied()))
                                                        .color(color)
                                                        .name(&format!("{}: {:.1}% ({} dmg)", 
                                                            name, 
                                                            percentage,
                                                            Self::format_damage(segment.value)))
                                                        .fill(0.5));
                                                }
                                            }
                                        }
                                    });
                            });
                        });
        
                        ui.vertical(|ui| {
                            ui.group(|ui| {
                                ui.heading("Total Damage by Character");
                                Plot::new("damage_bars")
                                    .legend(Legend::default())
                                    .height(300.0)
                                    .width(ui.available_width())
                                    .allow_drag(false)
                                    .allow_zoom(false)
                                    .y_axis_formatter(|y, _, _| Self::format_damage(y))
                                    .show(ui, |plot_ui| {
                                        if let Some(buffer) = self.data_buffer.try_lock() {
                                            let bars_data = Self::create_bar_data(&buffer);
                                            
                                            let bars: Vec<Bar> = bars_data
                                                .iter()
                                                .enumerate()
                                                .map(|(pos, (name, value, color_idx))| {
                                                    Bar::new(pos as f64, *value)
                                                        .name(name)
                                                        .fill(self.get_character_color(*color_idx))
                                                        .width(0.7)
                                                })
                                                .collect();
        
                                            let names: Vec<String> = bars_data.iter()
                                                .map(|(name, _, _)| name.clone())
                                                .collect();
        
                                            let chart = BarChart::new(bars)
                                                .element_formatter(Box::new(move |bar: &Bar, _: &BarChart| {
                                                    names[bar.argument as usize].clone()
                                                }));
        
                                            plot_ui.bar_chart(chart);
                                        }
                                    });
                            });
                        });
                    });
                });
            });
        });

        if self.show_connection_settings {
            egui::Window::new("Connection Settings")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Server:");
                        ui.text_edit_singleline(&mut self.server_addr);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Port:");
                        ui.text_edit_singleline(&mut self.server_port);
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Connect").clicked() {
                            self.show_connection_settings = false;
                            self.disconnect();
                            self.next_connect_attempt = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_connection_settings = false;
                        }
                    });
                });
        }

        if !self.connected {
            let now = std::time::Instant::now();
            
            if self.next_connect_attempt.map_or(true, |t| now >= t) {
                let addr = format!("{}:{}", self.server_addr, self.server_port);
                match self.network.connect(&addr) {
                    Ok(()) => {
                        let (tx, rx) = mpsc::channel(100);
                        self.rx = Some(rx);
                        if let Err(e) = self.network.start_receiving(tx) {
                            self.log_message(&format!("Failed to start receiver: {}", e));
                            self.next_connect_attempt = Some(now + std::time::Duration::from_secs(5));
                        } else {
                            self.connected = true;
                            self.next_connect_attempt = None;
                            self.log_message(&format!("Connected to {}", addr));
                        }
                    }
                    Err(_) => {
                        self.next_connect_attempt = Some(now + std::time::Duration::from_secs(5));
                    }
                }
            }
        }

        let packets = if let Some(rx) = &mut self.rx {
            let mut collected = Vec::new();
            while let Ok(packet) = rx.try_recv() {
                collected.push(packet);
            }
            collected
        } else {
            Vec::new()
        };
        
        for packet in packets {
            match packet.r#type.as_str() {
                "SetBattleLineup" => self.handle_lineup(&packet.data),
                "BattleBegin" => self.handle_battle_begin(&packet.data),
                "OnDamage" => self.handle_damage(&packet.data),
                "TurnEnd" => self.handle_turn_end(&packet.data), 
                "OnKill" => self.handle_kill(&packet.data),
                "BattleEnd" => self.handle_battle_end(),
                _ => self.log_message(&format!("Unknown packet type: {}", packet.r#type)),
            }
        }

        ctx.request_repaint();
    }
}

fn create_pie_slice(start_angle: f64, end_angle: f64) -> Vec<[f64; 2]> {
    let center = [0.0, 0.0];
    let radius = 0.8; 
    let mut points = vec![center];
    
    let steps = 50; 
    for i in 0..=steps {
        let angle = start_angle + (end_angle - start_angle) * (i as f64 / steps as f64);
        let (sin, cos) = angle.sin_cos();
        points.push([cos * radius, sin * radius]);
    }
    points.push(center);
    
    points
}

impl DamageAnalyzer {

    fn disconnect(&mut self) {
        self.network.disconnect();
        self.connected = false;
        self.rx = None;
        self.log_message("Disconnected");
    }

    fn toggle_pin(&mut self) {
        self.window_pinned = !self.window_pinned;
        self.log_message(if self.window_pinned {
            "Window pinned on top"
        } else {
            "Window unpinned"
        });
    }

    fn handle_lineup(&mut self, data: &serde_json::Value) {
        if let Ok(lineup_data) = serde_json::from_value::<SetupData>(data.clone()) {
            let names: Vec<String> = lineup_data.avatars.iter().map(|a| a.name.clone()).collect();
            
            fs::create_dir_all("damage_logs").unwrap_or_else(|e| {
                self.log_message(&format!("Failed to create damage_logs directory: {}", e));
            });

            let filename = format!("HSR_{}.csv", chrono::Local::now().format("%Y%m%d_%H%M%S"));
            let path = format!("damage_logs/{}", filename);
            
            match File::create(&path) {
                Ok(file) => {
                    self.csv_writer = Some(Writer::from_writer(file));
                    self.current_file = path.clone();
                    
                    if let Some(writer) = &mut self.csv_writer {
                        if let Err(e) = writer.write_record(&names) {
                            self.log_message(&format!("Failed to write CSV headers: {}", e));
                        }
                    }

                    if let Some(mut buffer) = self.data_buffer.try_lock() {
                        buffer.init_characters(&names);
                        buffer.rows.clear();
                    }

                    self.log_message(&format!("Created CSV: {}", filename));
                    self.log_message(&format!("Headers: {:?}", names));
                }
                Err(e) => {
                    self.log_message(&format!("Failed to create CSV file: {}", e));
                }
            }
        }
    }

    fn handle_battle_begin(&mut self, _data: &serde_json::Value) {
        self.log_message("Battle started");
    }

    fn handle_damage(&mut self, data: &serde_json::Value) {
        if let Ok(damage_data) = serde_json::from_value::<DamageData>(data.clone()) {
            let attacker = damage_data.attacker.name.clone();
            let damage = damage_data.damage;
            
            if damage > 0.0 {
                self.log_message(&format!("{} dealt {} damage", attacker, damage));
            }
            
            let mut should_write = false;
            let mut row = vec![0.0; if let Some(buffer) = self.data_buffer.try_lock() {
                buffer.column_names.len()
            } else {
                0
            }];
            
            if let Some(mut buffer) = self.data_buffer.try_lock() {
                if let Some(idx) = buffer.column_names.iter().position(|name| name == &attacker) {
                    row[idx] = damage;
                    *buffer.total_damage.entry(attacker.clone()).or_insert(0.0) += damage;
                    *buffer.current_turn.entry(attacker.clone()).or_insert(0.0) += damage;
                    should_write = true;
                }
                buffer.rows.push(row.clone());
            }
            
            if should_write {
                if let Some(writer) = &mut self.csv_writer {
                    let _ = writer.write_record(&row.iter().map(|&x| x.to_string()).collect::<Vec<_>>());
                    let _ = writer.flush();
                }
            }
        }
    }

    fn handle_kill(&mut self, data: &serde_json::Value) {
        if let Ok(kill_data) = serde_json::from_value::<KillData>(data.clone()) {
            self.log_message(&format!("{} has killed", kill_data.attacker.name));
        }
    }

    fn handle_battle_end(&mut self) {
        let final_turn_data = if let Some(mut buffer) = self.data_buffer.try_lock() {
            if !buffer.current_turn.is_empty() {
                let total_damage: f32 = buffer.current_turn.values().sum();
                let final_turn = buffer.current_turn.clone();
                let av = buffer.current_av;

                buffer.update_dpav(total_damage, av);
                buffer.turn_damage.push(final_turn.clone());

                Some((final_turn, total_damage))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((final_turn, total_damage)) = final_turn_data {
            for (name, damage) in final_turn {
                if damage > 0.0 {
                    self.log_message(&format!(
                        "Final turn summary - {}: {} damage",
                        name, damage
                    ));
                }
            }
            self.log_message(&format!("Final turn total damage: {}", total_damage));
        }

        self.csv_writer = None;
        self.log_message("Battle ended - CSV file closed");
        self.disconnect();
    }

    fn format_damage(value: f64) -> String {
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

    fn get_character_color(&self, index: usize) -> egui::Color32 {
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

    fn log_message(&mut self, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        let formatted = format!("[{}] {}", timestamp, message);
        self.log_messages.push(formatted);
        
        if self.log_messages.len() > 1000 {
            self.log_messages.remove(0);
        }
    }

    fn create_pie_segments(damage_map: &HashMap<String, f32>, column_names: &[String]) -> Vec<(String, PieSegment, usize)> {
        let total: f64 = damage_map.values().sum::<f32>() as f64;
        let mut segments = Vec::new();
        let mut start_angle = -std::f64::consts::FRAC_PI_2; 

        for (i, name) in column_names.iter().enumerate() {
            if let Some(&damage) = damage_map.get(name) {
                let fraction = damage as f64 / total;
                let angle = fraction * std::f64::consts::TAU;
                let end_angle = start_angle + angle;

                segments.push((name.clone(), PieSegment {
                    points: create_pie_slice(start_angle, end_angle),
                    value: damage as f64,
                }, i));

                start_angle = end_angle;
            }
        }

        segments
    }

    fn create_bar_data(buffer: &DataBufferInner) -> Vec<(String, f64, usize)> {
        
        let mut data: Vec<_> = buffer.column_names.iter()
            .enumerate()
            .filter_map(|(i, name)| {
                buffer.total_damage.get(name)
                    .map(|&damage| (name.clone(), damage as f64, i))
            })
            .collect();
        
        
        data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        data
    }

    fn handle_turn_end(&mut self, data: &serde_json::Value) {
        if let Ok(turn_data) = serde_json::from_value::<TurnData>(data.clone()) {
            for (avatar, &damage) in turn_data.avatars.iter().zip(turn_data.avatars_damage.iter()) {
                if damage > 0.0 {
                    self.log_message(&format!(
                        "Turn summary - {}: {} damage",
                        avatar.name, damage
                    ));
                }
            }
            self.log_message(&format!("Total turn damage: {}", turn_data.total_damage));
            
            if let Some(mut buffer) = self.data_buffer.try_lock() {
                buffer.current_av = turn_data.action_value;
                buffer.av_history.push(turn_data.action_value);
                buffer.update_dpav(turn_data.total_damage, turn_data.action_value);
                let current = buffer.current_turn.clone();
                buffer.turn_damage.push(current);
                buffer.current_turn.clear();
            }
        }
    }
}

struct PieSegment {
    points: Vec<[f64; 2]>,
    value: f64,
}

#[derive(Debug, Deserialize)]
struct KillData {
    attacker: Avatar,
}

#[derive(Debug, Deserialize)]
struct SetupData {
    avatars: Vec<Avatar>,
}
