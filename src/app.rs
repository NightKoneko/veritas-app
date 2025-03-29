use std::sync::{Arc, Mutex};
use std::fs::{self, File};
use std::time::Duration;
use eframe::egui::{self};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use csv::Writer;
use crate::core::{self, ConnectionStatus};
use crate::{models::*, network::NetworkClient};

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
    pub server_addr: Arc<Mutex<String>>,
    pub server_port: Arc<Mutex<String>>,
    pub connected: bool,
    pub data_buffer: Arc<DataBuffer>,
    pub log_messages: Vec<String>,
    pub payload_rx: mpsc::Receiver<Packet>,
    pub csv_writer: Option<Writer<File>>,
    pub current_file: String,
    pub window_pinned: bool,
    pub show_connection_settings: bool,
    pub show_preferences: bool,
    pub theme: Theme,
    pub connection_status_rx: mpsc::Receiver<ConnectionStatus>,
    pub is_sidebar_expanded: bool,
    pub runtime: Runtime
}

impl DamageAnalyzer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(Theme::Light.visuals());
        egui_material_icons::initialize(&cc.egui_ctx);

        let (status_tx, status_rx) = mpsc::channel(1);
        let (payload_tx, payload_rx) = mpsc::channel(100);

        let app = Self {
            server_addr: Mutex::new("127.0.0.1".to_string()).into(),
            server_port: Mutex::new("1305".to_string()).into(),
            connected: false,
            data_buffer: Arc::new(DataBuffer::new()),
            log_messages: Vec::new(),
            payload_rx,
            csv_writer: None,
            current_file: String::new(),
            window_pinned: false,
            show_connection_settings: false,
            show_preferences: false,
            theme: Theme::Light,
            connection_status_rx: status_rx,
            is_sidebar_expanded: false,
            runtime: Runtime::new().unwrap()
        };
            
        let server_addr = app.server_addr.clone();
        let server_port = app.server_port.clone();

        app.runtime.spawn(async move {
            core::start_connection(&payload_tx, &status_tx, &server_addr, &server_port).await;
        });

        app
    }

    pub fn set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.theme = theme;
        ctx.set_visuals(theme.visuals());
    }


    fn on_try_to_connect(&mut self) {
        if !self.connected {
            if let Some(status) = self.connection_status_rx.try_recv().ok() {
                match status {
                    ConnectionStatus::Connected => {
                        self.connected = true;
                        let addr = format!("{}:{}", self.server_addr.lock().unwrap(), self.server_port.lock().unwrap());
                        self.log_message(&format!("Connected to {}", addr));
                    }
                    ConnectionStatus::Failed(err) => {
                        self.connected = false;
                        self.log_message(&format!("Failed to connect: {}", err));
                    }
                }
            }
        }
    }

    fn handle_packets(&mut self) {
        match self.payload_rx.try_recv() {
            Ok(packet) => {
                match packet.r#type.as_str() {
                    "SetBattleLineup" => self.handle_lineup(&packet.data),
                    "BattleBegin" => self.handle_battle_begin(&packet.data),
                    "OnDamage" => self.handle_damage(&packet.data),
                    "TurnEnd" => self.handle_turn_end(&packet.data), 
                    "OnKill" => self.handle_kill(&packet.data),
                    "BattleEnd" => self.handle_battle_end(),
                    _ => self.log_message(&format!("Unknown packet type: {}", packet.r#type)),
                }    
            },
            Err(_) => {},
        }
    }
}

impl eframe::App for DamageAnalyzer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_menubar_panel(ctx, _frame);
        self.show_statusbar_panel(ctx, _frame);

        self.show_sidebar_panel(ctx, _frame);
        self.show_av_panel(ctx, _frame);

        self.show_central_panel(ctx, _frame);

        self.on_try_to_connect();
        self.handle_packets();    

        ctx.request_repaint();
    }
}


impl DamageAnalyzer {
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