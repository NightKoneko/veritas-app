use std::sync::Arc;
use std::time::Duration;
use eframe::egui::{self, Theme};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use crate::core::message_logger::MessageLogger;
use crate::core::packet_handler::PacketHandler;
use crate::core::models::*;
use crate::core::network::{ConnectionStatus, NetworkClient};

pub struct AppState {
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
    pub packet_handler: Arc<Mutex<PacketHandler>>,
    pub state: AppState,
    pub runtime: Runtime
}

impl DamageAnalyzer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_material_icons::initialize(&cc.egui_ctx);

        let message_logger = Arc::new(
            Mutex::new(
                MessageLogger::default()
            )
        );
        let data_buffer = Arc::new(DataBuffer::new());
        let packet_handler = Arc::new(
            Mutex::new(
                PacketHandler::new(message_logger.clone(), data_buffer.clone())
            )
        );

        let app = Self {
            server_addr: Mutex::new("127.0.0.1".to_string()).into(),
            server_port: Mutex::new("1305".to_string()).into(),
            connected: Mutex::new(false).into(),
            data_buffer,
            message_logger,
            packet_handler,
            state: AppState { 
                is_sidebar_expanded: false,
                is_window_pinned: false,
                show_connection_settings: false, 
                show_preferences: false
            },
            runtime: Runtime::new().unwrap()
        };

        // Enter the runtime so that `tokio::spawn` is available immediately.
        let _enter = app.runtime.enter();

        app.start_background_workers();

        app
    }

    fn start_background_workers(&self) {
        let (status_tx, status_rx) = mpsc::channel(1);
        let (payload_tx, payload_rx) = mpsc::channel(100);

        self.start_connection_worker(payload_tx, status_tx);
        self.start_logger_worker(payload_rx);
        self.start_connection_status_worker(status_rx);
    }

    fn start_logger_worker(&self, mut payload_rx: mpsc::Receiver<Packet>) {
        let packet_handler = self.packet_handler.clone();
        self.runtime.spawn(async move {
            loop {
                let mut packet_handler = packet_handler.lock().await;
                packet_handler.handle_packets(&mut payload_rx).await;
                drop(packet_handler);
                sleep(Duration::from_millis(10)).await;
            }
        });
    }
    
    fn start_connection_worker(&self, payload_tx: mpsc::Sender<Packet>, status_tx: mpsc::Sender<ConnectionStatus>) {
        let server_addr = self.server_addr.clone();
        let server_port = self.server_port.clone();

        self.runtime.spawn(async move {
            let mut network_client = NetworkClient::new();
            loop {
                network_client.start_connection(&payload_tx, &status_tx, &server_addr.clone(), &server_port.clone()).await;
                sleep(Duration::from_secs(2)).await;
            }
        });    
    }

    fn start_connection_status_worker(&self, mut status_rx: Receiver<ConnectionStatus>) {
        // This is kinda useless bc we don't know when the connection has been severed
        // For it to try to reconnect again
        let server_addr = self.server_addr.clone();
        let server_port = self.server_port.clone();
        
        let connected = self.connected.clone();
        let message_logger = self.message_logger.clone();

        self.runtime.spawn(async move {
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
                drop(connected_lock);
                sleep(Duration::from_secs(2)).await;
            }
        });    
    }
}

impl eframe::App for DamageAnalyzer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_menubar_panel(ctx, _frame);
        self.show_statusbar_panel(ctx, _frame);

        self.show_sidebar_panel(ctx, _frame);
        self.show_av_panel(ctx, _frame);

        self.show_central_panel(ctx, _frame);

        ctx.request_repaint_after(Duration::from_millis(50));
    }
}