use crate::core::message_logger::MessageLogger;
use crate::core::models::*;
use crate::core::network::{ConnectionStatus, NetworkClient};
use crate::core::packet_handler::PacketHandler;
use crate::core::config::Config;
use crate::core::updater::Updater;
use eframe::egui::{self};
use egui_toast::{ToastOptions, Toast, ToastKind, Toasts};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, timeout};

#[derive(PartialEq, Clone)]
pub enum Unit {
    Turn,
    ActionValue
}

pub struct UpdateState {
    pub downloaded: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub is_sidebar_expanded: bool,
    pub is_window_pinned: bool,
    pub show_connection_settings: bool,
    pub show_preferences: bool,
    pub show_launcher: bool,
    pub game_path: Option<String>, 
    pub graph_x_unit: Unit,
    pub config: Config,
    pub show_about: bool,
    pub show_updates: bool,
    pub checked_app_version: Option<String>,
    pub checked_dll_version: Option<String>,
    pub update_state: Arc<Mutex<UpdateState>>,
}

pub struct DamageAnalyzer {
    pub server_addr: Arc<Mutex<String>>,
    pub server_port: Arc<Mutex<String>>,
    pub connected: Arc<Mutex<bool>>,
    pub data_buffer: Arc<DataBuffer>,
    pub message_logger: Arc<Mutex<MessageLogger>>,
    pub is_there_update: Arc<Mutex<bool>>,
    pub state: AppState,
    pub runtime: Runtime,
    pub updater: Updater,
    pub toasts: Arc<Mutex<Toasts>>,
}

impl DamageAnalyzer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_material_icons::initialize(&cc.egui_ctx);
        egui_zhcn_fonts::add_sys_ui_fonts(&cc.egui_ctx);

        let message_logger = Arc::new(Mutex::new(MessageLogger::default()));
        let data_buffer = Arc::new(DataBuffer::new());
        let packet_handler = PacketHandler::new(
            message_logger.clone(),
            data_buffer.clone(),
        );

        let mut app = Self {
            server_addr: Mutex::new("127.0.0.1".to_string()).into(),
            server_port: Mutex::new("1305".to_string()).into(),
            connected: Mutex::new(false).into(),
            data_buffer,
            message_logger,
            is_there_update: Mutex::new(false).into(),
            state: AppState {
                is_sidebar_expanded: false,
                is_window_pinned: false,
                show_connection_settings: false,
                show_preferences: false,
                show_launcher: false,
                game_path: Config::load().game_path,
                graph_x_unit: Unit::Turn,
                config: Config::load(),
                show_about: false,
                show_updates: false,
                checked_app_version: None,
                checked_dll_version: None,
                update_state: Arc::new(Mutex::new(UpdateState {
                    downloaded: false,
                })),
            },
            runtime: Runtime::new().unwrap(),
            updater: Updater::new(),
            toasts: Arc::new(Mutex::new(Toasts::new()
                .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp))),
        };

        // Enter the runtime so that `tokio::spawn` is available immediately.
        let _enter = app.runtime.enter();

        app.start_background_workers(&cc.egui_ctx, packet_handler);

        let toasts = app.toasts.clone();
        let mut updater = app.updater.clone();
        let (version_tx, version_rx) = tokio::sync::oneshot::channel();
        
        app.runtime.spawn(async move {
            if let Ok(mut toast_lock) = toasts.try_lock() {
                toast_lock.add(Toast {
                    text: "Checking for updates...".into(),
                    kind: ToastKind::Info,
                    options: ToastOptions::default()
                        .duration_in_seconds(0.5),
                    ..Default::default()
                });
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if let Some(new_version) = updater.check_app_update().await {
                if let Ok(mut toast_lock) = toasts.try_lock() {
                    if new_version != env!("CARGO_PKG_VERSION") {
                        toast_lock.add(Toast {
                            text: format!("Veritas App version {} is available!", new_version).into(),
                            kind: ToastKind::Success,
                            options: ToastOptions::default()
                                .duration_in_seconds(5.0),
                            ..Default::default()
                        });
                        let _ = version_tx.send(Some(new_version));
                    } else {
                        toast_lock.add(Toast {
                            text: "Veritas App is up to date".into(),
                            kind: ToastKind::Info,
                            options: ToastOptions::default()
                                .duration_in_seconds(3.0),
                            ..Default::default()
                        });
                        let _ = version_tx.send(None);
                    }
                }
            }
            
            if let Ok(()) = updater.update_dll().await {
                if let Some(new_version) = updater.latest_dll_version() {
                    let current_version = updater.current_dll_version();
                    if current_version.is_none() || current_version.unwrap() != new_version {
                        if let Ok(mut toast_lock) = toasts.try_lock() {
                            toast_lock.add(Toast {
                                text: format!("Updated Veritas to version {}!", new_version).into(),
                                kind: ToastKind::Success,
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        });

        if let Ok(Some(new_version)) = version_rx.blocking_recv() {
            app.state.checked_app_version = Some(new_version);
        }

        app
    }

    fn start_background_workers(&self, ctx: &egui::Context, packet_handler: PacketHandler) {
        let (status_tx, status_rx) = mpsc::channel(1);
        let (payload_tx, payload_rx) = mpsc::channel(100);

        self.start_connection_worker(payload_tx, status_tx);
        self.start_packet_worker(payload_rx, ctx.clone(), packet_handler);
        self.start_connection_status_worker(status_rx);
    }

    fn start_packet_worker(&self, mut payload_rx: mpsc::Receiver<Packet>, ctx: egui::Context, mut packet_handler: PacketHandler) {
        let is_there_update = self.is_there_update.clone();
        self.runtime.spawn(async move {
            loop {
                let mut is_there_update_lock = is_there_update.lock().await;
                *is_there_update_lock = packet_handler.handle_packets(&mut payload_rx).await;
                if *is_there_update_lock && !ctx.has_requested_repaint() {
                    ctx.request_repaint();
                }
                drop(is_there_update_lock);
                sleep(Duration::from_millis(10)).await;
            }
        });
    }

    fn start_connection_worker(
        &self,
        payload_tx: mpsc::Sender<Packet>,
        status_tx: mpsc::Sender<ConnectionStatus>,
    ) {
        let server_addr = self.server_addr.clone();
        let server_port = self.server_port.clone();

        self.runtime.spawn(async move {

            let mut network_client = NetworkClient::new();
            // Try connecting
            loop {
                let is_connected = network_client
                    .start_connection(&status_tx, &server_addr, &server_port)
                    .await;

                if is_connected {
                    // let orig_server_addr = (*server_addr.lock().await).clone();
                    // let orig_server_port = (*server_port.lock().await).clone();

                    static MAX_RETRIES: usize = 2;
                    static INITIAL_TIMEOUT: Duration = Duration::from_secs(1);
                    let mut retries = 0;
                    let mut timeout_duration = INITIAL_TIMEOUT;
                    // Try receiving packets
                    loop {
                        // Why is this not working??
                        // if orig_server_addr != *server_addr.lock().await || orig_server_port != *server_addr.lock().await {
                        //     network_client.disconnect().await;
                        //     break;
                        // }
                        match timeout(timeout_duration, network_client.start_receiving(&payload_tx)).await {
                            // If no timeout
                            Ok(v) => {
                                // If not a packet
                                if v.is_err() {
                                    if !network_client.try_pinging(
                                        &mut retries,
                                        MAX_RETRIES,
                                        &mut timeout_duration,
                                        &INITIAL_TIMEOUT
                                    ).await {
                                        break;
                                    }
                                }
                                // Reset if packet
                                else {
                                    retries = 0;
                                    timeout_duration = INITIAL_TIMEOUT;
                                    sleep(Duration::from_millis(10)).await;
                                }
                            }
                            // If timeout
                            Err(_) => {
                                if !network_client.try_pinging(
                                    &mut retries,
                                    MAX_RETRIES,
                                    &mut timeout_duration,
                                    &INITIAL_TIMEOUT
                                ).await {
                                    break;
                                }
                            }
                        }
                    }
                }
                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    fn start_connection_status_worker(&self, mut status_rx: Receiver<ConnectionStatus>) {
        let server_addr = self.server_addr.clone();
        let server_port = self.server_port.clone();

        let connected = self.connected.clone();
        let message_logger = self.message_logger.clone();

        self.runtime.spawn(async move {
            loop {
                let mut connected_lock = connected.lock().await;
                if let Some(status) = status_rx.try_recv().ok() {
                    match status {
                        ConnectionStatus::Connected => {
                            let mut message_logger_lock = message_logger.lock().await;
                            *connected_lock = true;
                            let addr = format!(
                                "{}:{}",
                                server_addr.lock().await,
                                server_port.lock().await
                            );
                            message_logger_lock.log(&format!("Connected to {}", addr));
                        }
                        ConnectionStatus::Failed(_err) => {
                            *connected_lock = false;
                            // Unsure how to handle this well
                            // message_logger_lock.log(&format!("Failed to connect: {}", err));
                        }
                    }
                }
                drop(connected_lock);
                sleep(Duration::from_secs(1)).await;
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
        
        if let Ok(mut toasts) = self.toasts.try_lock() {
            toasts.show(ctx);
        }
    }
}