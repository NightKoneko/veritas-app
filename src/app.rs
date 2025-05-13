use crate::core::config::Config;
use crate::core::message_logger::MessageLogger;
use crate::core::models::*;
use crate::core::packet_handler::PacketHandler;
use crate::core::updater::Updater;
use eframe::egui::{self};
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use futures_util::FutureExt;
use rust_socketio::Event;
use rust_socketio::{
    asynchronous::{Client, ClientBuilder},
    Payload,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;

#[derive(PartialEq, Clone)]
pub enum Unit {
    Turn,
    ActionValue,
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
        let packet_handler = PacketHandler::new(message_logger.clone(), data_buffer.clone());

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
                update_state: Arc::new(Mutex::new(UpdateState { downloaded: false })),
            },
            runtime: Runtime::new().unwrap(),
            updater: Updater::new(),
            toasts: Arc::new(Mutex::new(
                Toasts::new()
                    .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                    .direction(egui::Direction::BottomUp),
            )),
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
                    options: ToastOptions::default().duration_in_seconds(0.5),
                    ..Default::default()
                });
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if let Some(new_version) = updater.check_app_update().await {
                if let Ok(mut toast_lock) = toasts.try_lock() {
                    if new_version != env!("CARGO_PKG_VERSION") {
                        toast_lock.add(Toast {
                            text: format!("Veritas App version {} is available!", new_version)
                                .into(),
                            kind: ToastKind::Success,
                            options: ToastOptions::default().duration_in_seconds(5.0),
                            ..Default::default()
                        });
                        let _ = version_tx.send(Some(new_version));
                    } else {
                        toast_lock.add(Toast {
                            text: "Veritas App is up to date".into(),
                            kind: ToastKind::Info,
                            options: ToastOptions::default().duration_in_seconds(3.0),
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
                                options: ToastOptions::default().duration_in_seconds(5.0),
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
        let (payload_tx, payload_rx) = mpsc::channel(100);
        self.start_packet_worker(payload_rx, ctx.clone(), packet_handler);
        self.start_client_worker(payload_tx);
    }

    fn start_packet_worker(
        &self,
        mut payload_rx: mpsc::Receiver<Packet>,
        ctx: egui::Context,
        mut packet_handler: PacketHandler,
    ) {
        let is_there_update = self.is_there_update.clone();
        self.runtime.spawn(async move {
            loop {
                let mut is_there_update_lock = is_there_update.lock().await;
                *is_there_update_lock = packet_handler.handle_packets(&mut payload_rx).await;
                if *is_there_update_lock && !ctx.has_requested_repaint() {
                    ctx.request_repaint();
                }
                drop(is_there_update_lock);
                sleep(Duration::from_millis(1)).await;
            }
        });
    }

    fn start_client_worker(
        &self,
        payload_tx: mpsc::Sender<Packet>,
    ) {
        let server_addr = self.server_addr.clone();
        let server_port = self.server_port.clone();
        let connected = self.connected.clone();

        // This is so verbose, but necessary
        self.runtime.spawn(async move {
            loop {
                if !*connected.lock().await {
                    let on_connected_status = connected.clone();
                    let on_disconnected_status = connected.clone();
                    let payload_tx = payload_tx.clone();

                    let connected_callback = move |_payload: Payload, _socket: Client| {
                        let on_connected_status = on_connected_status.clone();
                        async move {
                            *on_connected_status.lock().await = true;
                        }
                        .boxed()
                    };

                    let disconnected_callback = move |_payload: Payload, _socket: Client| {
                        let on_disconnected_status = on_disconnected_status.clone();
                        async move {
                            *on_disconnected_status.lock().await = false;
                        }
                        .boxed()
                    };

                    let message_handler_callback  = move |event: Event, payload: Payload, _socket: Client| {
                        let payload_tx = payload_tx.clone();
                        async move {
                            if let Event::Custom(e) = event {
                                if let Payload::Text(text) = payload {
                                    for msg in text {
                                        let _ = payload_tx.send(Packet { r#type: e.clone(), data: msg }).await;
                                    }
                                }
                            }
                        }
                        .boxed()
                    };

                    ClientBuilder::new(format!(
                        "http://{}:{}/",
                        &server_addr.lock().await,
                        &server_port.lock().await
                    ))
                    .namespace("/")
                    .on(Event::Connect, connected_callback)
                    .on(Event::Error, disconnected_callback)
                    .on_any(message_handler_callback)
                    .reconnect(false)
                    .connect()
                    .await
                    .ok();
                }

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
