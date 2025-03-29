use std::sync::Arc;
use std::fs::{self, File};
use std::collections::HashMap;
use eframe::egui::{self};
use serde::Deserialize;
use tokio::sync::mpsc;
use csv::Writer;
use crate::{models::*, network::NetworkClient};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Dark,
    Light
}

impl Theme {
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }

    pub fn visuals(&self) -> egui::Visuals {
        match self {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        }
    }

    pub const ALL: &'static [Theme] = &[Theme::Dark, Theme::Light];
}

pub struct DamageAnalyzer {
    pub server_addr: String,
    pub server_port: String,
    pub network: NetworkClient,
    pub connected: bool,
    pub data_buffer: Arc<DataBuffer>,
    pub log_messages: Vec<String>,
    pub rx: Option<mpsc::Receiver<Packet>>,
    pub csv_writer: Option<Writer<File>>,
    pub current_file: String,
    pub window_pinned: bool,
    pub show_connection_settings: bool,
    pub show_preferences: bool,
    pub theme: Theme,
    pub connection_status_rx: Option<mpsc::Receiver<ConnectionStatus>>,
    pub is_sidebar_expanded: bool
}

#[derive(Debug)]
pub enum ConnectionStatus {
    Connected(mpsc::Receiver<Packet>),
    Failed(String),
}

impl DamageAnalyzer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(Theme::Light.visuals());
        egui_material_icons::initialize(&cc.egui_ctx);

        let mut instance = Self {
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
            show_connection_settings: false,
            show_preferences: false,
            theme: Theme::Light,
            connection_status_rx: None,
            is_sidebar_expanded: false
        };

        instance.start_connection_thread();
        
        instance
    }

    pub fn set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.theme = theme;
        ctx.set_visuals(theme.visuals());
    }

    fn start_connection_thread(&mut self) {
        let server_addr = self.server_addr.clone();
        let server_port = self.server_port.clone();
        let (status_tx, status_rx) = mpsc::channel(1);
        
        thread::spawn(move || {
            loop {
                let addr = format!("{}:{}", server_addr, server_port);
                let mut client = NetworkClient::new();
                
                match client.connect(&addr) {
                    Ok(()) => {
                        let (tx, rx) = mpsc::channel(100);
                        match client.start_receiving(tx) {
                            Ok(()) => {
                                if status_tx.blocking_send(ConnectionStatus::Connected(rx)).is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                if status_tx.blocking_send(ConnectionStatus::Failed(e.to_string())).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        });
        
        self.connection_status_rx = Some(status_rx);
    }
}

impl eframe::App for DamageAnalyzer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_menubar_panel(ctx, _frame);
        self.show_statusbar_panel(ctx, _frame);

        self.show_sidebar_panel(ctx, _frame);
        self.show_av_panel(ctx, _frame);

        self.show_central_panel(ctx, _frame);

        if !self.connected {
            if let Some(status) = self.connection_status_rx.as_mut().and_then(|rx| rx.try_recv().ok()) {
                match status {
                    ConnectionStatus::Connected(packet_rx) => {
                        self.rx = Some(packet_rx);
                        self.connected = true;
                        let addr = format!("{}:{}", self.server_addr, self.server_port);
                        self.log_message(&format!("Connected to {}", addr));
                    }
                    ConnectionStatus::Failed(err) => {
                        self.log_message(&format!("Failed to connect: {}", err));
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
                "TurnBegin" => self.handle_turn_begin(&packet.data),
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
    let p = (end_angle - start_angle)/(steps as f64);
    for i in 0..=steps {
        let angle = start_angle + p*i as f64;
        let (sin, cos) = angle.sin_cos();
        points.push([cos * radius, sin * radius]);
    }
    points.push(center);
    
    points
}

impl DamageAnalyzer {

    pub fn disconnect(&mut self) {
        self.network.disconnect();
        self.connected = false;
        self.rx = None;
        self.start_connection_thread();
        self.log_message("Disconnected");
    }

    pub fn toggle_pin(&mut self) {
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

    pub fn get_character_color(&self, index: usize) -> egui::Color32 {
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

    pub fn create_pie_segments(damage_map: &HashMap<String, f32>, column_names: &[String]) -> Vec<(String, PieSegment, usize)> {
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

    pub fn create_bar_data(buffer: &DataBufferInner) -> Vec<(String, f64, usize)> {
        buffer.column_names.iter()
            .enumerate()
            .filter_map(|(i, name)| {
                buffer.total_damage.get(name)
                    .map(|&damage| (name.clone(), damage as f64, i))
            })
            .collect()
    }

    fn handle_turn_begin(&mut self, data: &serde_json::Value) {
        if let Ok(turn_data) = serde_json::from_value::<TurnBeginData>(data.clone()) {
            if let Some(mut buffer) = self.data_buffer.try_lock() {
                buffer.current_av = turn_data.action_value;
            }
            self.log_message(&format!("Turn begin - AV: {:.2}", turn_data.action_value));
        }
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
                let turn_total: f32 = turn_data.total_damage;
                let current_av = buffer.current_av;
                
                buffer.av_history.push(current_av);
                
                if current_av > 0.0 {
                    buffer.update_dpav(turn_total, current_av);
                }
                
                let current = buffer.current_turn.clone();
                buffer.turn_damage.push(current);
                buffer.current_turn.clear();
            }
        }
    }
}

pub struct PieSegment {
    pub points: Vec<[f64; 2]>,
    pub value: f64,
}

#[derive(Debug, Deserialize)]
struct KillData {
    attacker: Avatar,
}

#[derive(Debug, Deserialize)]
struct SetupData {
    avatars: Vec<Avatar>,
}
