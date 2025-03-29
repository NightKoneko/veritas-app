use std::sync::{Arc};
use std::fs::{self, File};
use eframe::egui::{self};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, Mutex};
use csv::Writer;
use crate::message_logger::MessageLogger;
use crate::packet_handler::PacketHandler;
use crate::{models::*, packet_handler};
use crate::network::{ConnectionStatus, NetworkClient};

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
    pub connected: Arc<Mutex<bool>>,
    pub data_buffer: Arc<DataBuffer>,
    pub message_logger: Arc<Mutex<MessageLogger>>,
    pub window_pinned: bool,
    pub show_connection_settings: bool,
    pub show_preferences: bool,
    pub theme: Theme,
    pub is_sidebar_expanded: bool,
    pub runtime: Runtime
}

impl DamageAnalyzer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(Theme::Light.visuals());
        egui_material_icons::initialize(&cc.egui_ctx);
        let (status_tx, mut status_rx) = mpsc::channel(1);
        let (payload_tx, mut payload_rx) = mpsc::channel(100);

        let app = Self {
            server_addr: Mutex::new("127.0.0.1".to_string()).into(),
            server_port: Mutex::new("1305".to_string()).into(),
            connected: Mutex::new(false).into(),
            data_buffer: DataBuffer::new().into(),
            message_logger: Mutex::new(MessageLogger::default()).into(),
            window_pinned: false,
            show_connection_settings: false,
            show_preferences: false,
            theme: Theme::Light,
            is_sidebar_expanded: false,
            runtime: Runtime::new().unwrap()
        };
        
        {
            let server_addr = app.server_addr.clone();
            let server_port = app.server_port.clone();
    
            app.runtime.spawn(async move {
                let mut network_client = NetworkClient::new();
                network_client.start_connecting(&payload_tx, &status_tx, &server_addr.clone(), &server_port.clone()).await;
            });    
        }

        {
            let server_addr = app.server_addr.clone();
            let server_port = app.server_port.clone();
            let connected = app.connected.clone();
            let message_logger = app.message_logger.clone();

            app.runtime.spawn(async move {
                loop {
                    let mut connected_lock = connected.lock().await;
                    if !*connected_lock {
                        if let Some(status) = status_rx.try_recv().ok() {
                            let mut message_logger_lock = message_logger.lock().await;
                            match status {
                                ConnectionStatus::Connected => {
                                    *connected_lock = true;
                                    let addr = format!("{}:{}", server_addr.lock().await, server_port.lock().await);
                                    message_logger_lock.log_message(&format!("Connected to {}", addr));
                                }
                                ConnectionStatus::Failed(err) => {
                                    *connected_lock = false;
                                    message_logger_lock.log_message(&format!("Failed to connect: {}", err));
                                }
                            }
                        }
                    }        
                }
            });    
        }

        {
            let message_logger = app.message_logger.clone();
            let data_buffer = app.data_buffer.clone();

            app.runtime.spawn(async move {
                let mut packet_handler = PacketHandler::new(message_logger, data_buffer);
                loop {
                    packet_handler.handle_packets(&mut payload_rx).await;
                }
            });
        }

        app
    }

    pub fn set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.theme = theme;
        ctx.set_visuals(theme.visuals());
    }

}

impl eframe::App for DamageAnalyzer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_menubar_panel(ctx, _frame);
        self.show_statusbar_panel(ctx, _frame);

        self.show_sidebar_panel(ctx, _frame);
        self.show_av_panel(ctx, _frame);

        self.show_central_panel(ctx, _frame);

        ctx.request_repaint();
    }
}


impl DamageAnalyzer {
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

}
