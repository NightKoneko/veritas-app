use std::sync::Arc;
use eframe::egui::{self};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, Mutex};
use crate::core::message_logger::MessageLogger;
use crate::core::packet_handler::PacketHandler;
use crate::core::models::*;
use crate::core::network::{ConnectionStatus, NetworkClient};

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

pub struct AppState {
    pub theme: Theme,
    pub is_sidebar_expanded: bool,
    pub is_window_pinned: bool,
    pub show_connection_settings: bool,
    pub show_preferences: bool
}

pub struct DamageAnalyzer {
    pub server_addr: Arc<Mutex<String>>,
    pub server_port: Arc<Mutex<String>>,
    pub connected: Arc<Mutex<bool>>,
    pub data_buffer: Arc<DataBuffer>,
    pub message_logger: Arc<Mutex<MessageLogger>>,
    pub state: AppState,
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
            state: AppState { 
                theme: Theme::Light,
                is_sidebar_expanded: false,
                is_window_pinned: false,
                show_connection_settings: false, 
                show_preferences: false
            },
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
            // This is kinda useless bc we don't know when the connection has been severed
            // For it to try to reconnect again
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
                                    message_logger_lock.log(&format!("Connected to {}", addr));
                                }
                                ConnectionStatus::Failed(err) => {
                                    *connected_lock = false;
                                    message_logger_lock.log(&format!("Failed to connect: {}", err));
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
        self.state.theme = theme;
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