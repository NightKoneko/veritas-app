use eframe::egui::{self, RichText, ThemePreference, Hyperlink};
use rfd::FileDialog;
use std::fs;
use egui_toast::{Toast, ToastKind};
use egui_toast::ToastOptions;

use crate::{app::DamageAnalyzer, core::launcher::{hijack_process, start_hijacked_process}};
use crate::core::updater::VeritasVersion;

impl DamageAnalyzer {
    pub fn show_menubar_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Launch Game...").clicked() {
                        self.state.show_launcher = true;
                        ui.close_menu();
                    }
                    if ui.button("Connection Settings...").clicked() {
                        self.state.show_connection_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("Preferences...").clicked() {
                        self.state.show_preferences = true;
                        ui.close_menu();
                    }
                    
                    ui.separator();
                    
                    if ui.button("Updates...").clicked() {
                        self.state.show_updates = true;
                        ui.close_menu();
                    }

                    ui.separator();
                    
                    if ui.button("About...").clicked() {
                        self.state.show_about = true;
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("Tools", |ui| {
                    if ui.button("Spawn Server").clicked() {
                        hijack_process("StarRail", "veritas.dll");
                    }
                });
            });
        });

        if self.state.show_updates {
            egui::Window::new("Updates")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.heading("Veritas App");
                            ui.vertical(|ui| {
                                ui.label(format!("Current Version: {}", env!("CARGO_PKG_VERSION")));
                                
                                let latest_app_version = self.updater.latest_app_version();
                                let latest_version = self.state.checked_app_version.as_ref()
                                    .or(latest_app_version.as_ref());
                                
                                if let Some(latest_app) = latest_version {
                                    if latest_app != env!("CARGO_PKG_VERSION") {
                                        ui.label(RichText::new(format!("Available Version: {}", latest_app))
                                            .color(egui::Color32::GREEN));
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Check for Updates").clicked() {
                                    let toasts = self.toasts.clone();
                                    let mut updater = self.updater.clone();
                                    let (tx, rx) = tokio::sync::oneshot::channel();
                                    
                                    self.runtime.spawn(async move {
                                        if let Ok(mut toast_lock) = toasts.try_lock() {
                                            toast_lock.add(Toast {
                                                text: "Checking for updates...".into(),
                                                kind: ToastKind::Info,
                                                options: ToastOptions::default()
                                                    .duration_in_seconds(2.0),
                                                ..Default::default()
                                            });
                                        }

                                        if let Some(new_version) = updater.check_app_update().await {
                                            let _ = tx.send(new_version.clone());
                                            
                                            if new_version != env!("CARGO_PKG_VERSION") {
                                                if let Ok(mut toast_lock) = toasts.try_lock() {
                                                    toast_lock.add(Toast {
                                                        text: format!("Veritas App v{} is available!", new_version).into(),
                                                        kind: ToastKind::Success,
                                                        options: ToastOptions::default()
                                                            .duration_in_seconds(5.0),
                                                        ..Default::default()
                                                    });
                                                }
                                            } else {
                                                if let Ok(mut toast_lock) = toasts.try_lock() {
                                                    toast_lock.add(Toast {
                                                        text: "Veritas App is up to date".into(),
                                                        kind: ToastKind::Info,
                                                        options: ToastOptions::default()
                                                            .duration_in_seconds(3.0),
                                                        ..Default::default()
                                                    });
                                                }
                                            }
                                        }
                                    });

                                    if let Ok(new_version) = rx.blocking_recv() {
                                        self.state.checked_app_version = Some(new_version);
                                    }
                                }

                                let update_available = self.state.checked_app_version.as_ref()
                                    .or(self.updater.latest_app_version().as_ref())
                                    .map(|v| v != env!("CARGO_PKG_VERSION"))
                                    .unwrap_or(false);

                                let update_state = self.state.update_state.clone();
                                
                                ui.add_enabled_ui(update_available, |ui| {
                                    if ui.button("Download Update").clicked() {
                                        let toasts = self.toasts.clone();
                                        let mut updater = self.updater.clone();
                                        let update_state = update_state.clone();
                                        
                                        self.runtime.spawn(async move {
                                            if let Some(_) = updater.download_update().await {
                                                if let Ok(mut toast_lock) = toasts.try_lock() {
                                                    toast_lock.add(Toast {
                                                        text: "Update downloaded. Click 'Restart to Apply' to apply.".into(),
                                                        kind: ToastKind::Success,
                                                        options: ToastOptions::default()
                                                            .duration_in_seconds(30.0),
                                                        ..Default::default()
                                                    });
                                                }
                                                if let Ok(mut state) = update_state.try_lock() {
                                                    state.downloaded = true;
                                                }
                                            }
                                        });
                                    }
                                });

                                let update_downloaded = self.state.update_state.try_lock()
                                    .map(|state| state.downloaded)
                                    .unwrap_or(false);

                                ui.add_enabled_ui(update_downloaded, |ui| {
                                    if ui.button("Restart to Apply").clicked() {
                                        if let Ok(current_exe) = std::env::current_exe() {
                                            println!("Current exe: {:?}", current_exe);
                                            let parent = current_exe.parent().unwrap();
                                            let file_name = current_exe.file_name().unwrap().to_str().unwrap();

                                            let possible_paths = vec![
                                                parent.join(format!("{}.new", file_name)),
                                                parent.join(format!("{}.old", file_name)),
                                                parent.join("veritas.exe.new"),
                                                parent.join("veritas.exe.old"),
                                                current_exe.with_extension("new"),
                                                current_exe.with_extension("old"),
                                            ];

                                            println!("Checking possible update files:");
                                            for path in &possible_paths {
                                                println!("  {:?} (exists={})", path, path.exists());
                                            }

                                            if let Some(update_path) = possible_paths.iter().find(|p| p.exists()) {
                                                println!("Found update at: {:?}", update_path);
                                                
                                                std::thread::sleep(std::time::Duration::from_millis(500));

                                                let current_exe = std::env::current_exe().unwrap();
                                                let backup_path = current_exe.with_extension("old");

                                                if let Err(e) = fs::rename(&current_exe, &backup_path) {
                                                    println!("Failed to backup current exe: {}", e);
                                                    if let Ok(mut toast_lock) = self.toasts.try_lock() {
                                                        toast_lock.add(Toast {
                                                            text: format!("Update failed: {}", e).into(),
                                                            kind: ToastKind::Error,
                                                            options: ToastOptions::default()
                                                                .duration_in_seconds(5.0),
                                                            ..Default::default()
                                                        });
                                                    }
                                                    return;
                                                }

                                                if let Err(e) = fs::rename(update_path, &current_exe) {
                                                    println!("Failed to move new exe: {}", e);
                                                    let _ = fs::rename(&backup_path, &current_exe);
                                                    if let Ok(mut toast_lock) = self.toasts.try_lock() {
                                                        toast_lock.add(Toast {
                                                            text: format!("Update failed: {}", e).into(),
                                                            kind: ToastKind::Error,
                                                            options: ToastOptions::default()
                                                                .duration_in_seconds(5.0),
                                                            ..Default::default()
                                                        });
                                                    }
                                                    return;
                                                }

                                                match std::process::Command::new(&current_exe)
                                                    .args(std::env::args().skip(1))
                                                    .spawn() 
                                                {
                                                    Ok(_) => {
                                                        println!("Successfully launched new version");
                                                        if let Err(e) = fs::remove_file(&backup_path) {
                                                            println!("Failed to remove old version: {}", e);
                                                        } else {
                                                            println!("Cleaned up old version");
                                                        }
                                                        std::process::exit(0);
                                                    },
                                                    Err(e) => {
                                                        println!("Failed to launch new version: {}", e);
                                                        let _ = fs::rename(&backup_path, &current_exe);
                                                        if let Ok(mut toast_lock) = self.toasts.try_lock() {
                                                            toast_lock.add(Toast {
                                                                text: format!("Failed to restart: {}", e).into(),
                                                                kind: ToastKind::Error,
                                                                options: ToastOptions::default()
                                                                    .duration_in_seconds(5.0),
                                                                ..Default::default()
                                                            });
                                                        }
                                                    }
                                                }
                                            } else {
                                                println!("No update file found");
                                                if let Ok(mut toast_lock) = self.toasts.try_lock() {
                                                    toast_lock.add(Toast {
                                                        text: "Could not find update file".into(),
                                                        kind: ToastKind::Error,
                                                        options: ToastOptions::default()
                                                            .duration_in_seconds(5.0),
                                                        ..Default::default()
                                                    });
                                                }
                                            }
                                        }
                                    }
                                });
                            });
                        });

                        ui.add_space(8.0);

                        ui.group(|ui| {
                            ui.heading("Veritas");
                            ui.horizontal(|ui| {
                                ui.label("Version Type:");
                                egui::ComboBox::new("version_type", "")
                                    .selected_text(match self.updater.current_version_type {
                                        VeritasVersion::GlobalBeta => "Global Beta",
                                        VeritasVersion::CnBeta => "CN Beta",
                                        VeritasVersion::GlobalProd => "Global Prod",
                                    })
                                    .show_ui(ui, |ui| {
                                        let mut changed = false;
                                        ui.selectable_value(&mut self.updater.current_version_type, 
                                            VeritasVersion::GlobalBeta, "Global Beta").clicked()
                                            .then(|| changed = true);
                                        ui.selectable_value(&mut self.updater.current_version_type, 
                                            VeritasVersion::CnBeta, "CN Beta").clicked()
                                            .then(|| changed = true);
                                        ui.selectable_value(&mut self.updater.current_version_type, 
                                            VeritasVersion::GlobalProd, "Global Prod").clicked()
                                            .then(|| changed = true);
                                        
                                        if changed {
                                            self.state.config.version_type = Some(self.updater.current_version_type.clone());
                                            self.state.config.save();
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    if let Some(current_version) = self.updater.current_dll_version() {
                                        ui.label(format!("Current Version: {}", current_version));
                                        
                                        let latest_version = self.state.checked_dll_version.clone()
                                            .or_else(|| self.updater.latest_dll_version());
                                            
                                        if let Some(latest_dll) = latest_version {
                                            if latest_dll != current_version {
                                                ui.label(RichText::new(format!("Available Version: {}", latest_dll))
                                                    .color(egui::Color32::GREEN));
                                            }
                                        }
                                    } else {
                                        ui.label("Not installed");
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Check for Updates").clicked() {
                                    let toasts = self.toasts.clone();
                                    let mut updater = self.updater.clone();
                                    let config = self.state.config.clone();
                                    let (tx, rx) = tokio::sync::oneshot::channel();
                                    
                                    self.runtime.spawn(async move {
                                        if let Ok(mut toast_lock) = toasts.try_lock() {
                                            toast_lock.add(Toast {
                                                text: "Checking for updates...".into(),
                                                kind: ToastKind::Info,
                                                options: ToastOptions::default()
                                                    .duration_in_seconds(2.0),
                                                ..Default::default()
                                            });
                                        }

                                        if let Some(new_version) = updater.check_dll_update().await {
                                            let _ = tx.send((new_version.clone(), updater));
                                            if new_version != config.dll_version.unwrap_or_default() {
                                                if let Ok(mut toast_lock) = toasts.try_lock() {
                                                    toast_lock.add(Toast {
                                                        text: format!("Veritas version {} is available!", new_version).into(),
                                                        kind: ToastKind::Success,
                                                        options: ToastOptions::default()
                                                            .duration_in_seconds(5.0),
                                                        ..Default::default()
                                                    });
                                                }
                                            } else {
                                                if let Ok(mut toast_lock) = toasts.try_lock() {
                                                    toast_lock.add(Toast {
                                                        text: "Veritas is up to date".into(),
                                                        kind: ToastKind::Info,
                                                        options: ToastOptions::default()
                                                            .duration_in_seconds(3.0),
                                                        ..Default::default()
                                                    });
                                                }
                                            }
                                        }
                                    });

                                    if let Ok((new_version, updated_updater)) = rx.blocking_recv() {
                                        self.state.checked_dll_version = Some(new_version);
                                        self.updater = updated_updater;
                                    }
                                }

                                let current_version = self.updater.current_dll_version();
                                let checked_version = self.state.checked_dll_version.clone();
                                
                                let update_available = match (current_version, checked_version) {
                                    (Some(current), Some(checked)) => checked != current,
                                    (None, Some(_)) => true,
                                    _ => false,
                                };

                                ui.add_enabled_ui(update_available, |ui| {
                                    if ui.button("Download Update").clicked() {
                                        let toasts = self.toasts.clone();
                                        let mut updater = self.updater.clone();
                                        let config = self.state.config.clone();
                                        let (tx, rx) = tokio::sync::oneshot::channel();
                                        
                                        self.runtime.spawn(async move {
                                            match updater.update_dll().await {
                                                Ok(()) => {
                                                    if let Some(new_version) = updater.latest_dll_version() {
                                                        let mut config = config.clone();
                                                        config.dll_version = Some(new_version.clone());
                                                        config.save();

                                                        let _ = tx.send(config);

                                                        if let Ok(mut toast_lock) = toasts.try_lock() {
                                                            toast_lock.add(Toast {
                                                                text: format!("Veritas version {} has been downloaded!", new_version).into(),
                                                                kind: ToastKind::Success,
                                                                options: ToastOptions::default()
                                                                    .duration_in_seconds(5.0),
                                                                ..Default::default()
                                                            });
                                                        }
                                                    }
                                                },
                                                Err(e) => {
                                                    if let Ok(mut toast_lock) = toasts.try_lock() {
                                                        toast_lock.add(Toast {
                                                            text: format!("Failed to download update: {}", e).into(),
                                                            kind: ToastKind::Error,
                                                            options: ToastOptions::default()
                                                                .duration_in_seconds(5.0),
                                                            ..Default::default()
                                                        });
                                                    }
                                                }
                                            }
                                        });

                                        if let Ok(new_config) = rx.blocking_recv() {
                                            self.state.config = new_config;
                                            self.state.checked_dll_version = None;
                                            ctx.request_repaint();
                                        }
                                    }
                                });
                            });
                        });

                        ui.add_space(8.0);
                        if ui.button("Close").clicked() {
                            self.state.show_updates = false;
                        }
                    });
                });
        }

        if self.state.show_connection_settings {
            egui::Window::new("Connection Settings")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Server:");
                        let binding = self.server_addr.clone();
                        let mut server_addr = binding.blocking_lock();
                        ui.text_edit_singleline(&mut (*server_addr));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Port:");
                        let binding = self.server_port.clone();
                        let mut server_port = binding.blocking_lock();
                        ui.text_edit_singleline(&mut (*server_port));
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.state.show_connection_settings = false;
                        }
                    });
                });
        }

        if self.state.show_preferences {
            egui::Window::new("Preferences")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        let mut pref = ThemePreference::from(ctx.theme());
                        pref.radio_buttons(ui);
                        ctx.set_theme(pref);
                    });

                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.state.show_preferences = false;
                    }
                });
        }
        
        if self.state.show_launcher {
            egui::Window::new("Launch Game")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let path_text = self.state.game_path
                            .as_ref()
                            .map(|p| p.to_string())
                            .unwrap_or_else(|| "No game selected".to_string());
                        
                        ui.label("Game Path:");
                        ui.label(&path_text);
                        
                        if ui.button("Browse").clicked() {
                            if let Some(path) = FileDialog::new()
                                .add_filter("Executable", &["exe"])
                                .pick_file() 
                            {
                                let path_str = path.to_string_lossy().to_string();
                                self.state.game_path = Some(path_str.clone());
                                self.state.config.game_path = Some(path_str);
                                self.state.config.save();
                            }
                        }
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Launch").clicked() {
                            if let Some(game_path) = &self.state.game_path {
                                let dll_path = self.updater.get_dll_path();
                                start_hijacked_process(game_path, dll_path.to_str().unwrap());
                                self.state.show_launcher = false;
                            }
                        }

                        if ui.button("Cancel").clicked() {
                            self.state.show_launcher = false;
                        }
                    });
                });
        }

        if self.state.show_about {
            egui::Window::new("About Veritas")
                .collapsible(false)
                .resizable(false)
                .min_width(400.0)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(8.0);
                        ui.heading("Veritas App Version");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        
                        if let Some(latest_app) = self.updater.latest_app_version() {
                            if latest_app != env!("CARGO_PKG_VERSION") {
                                ui.label(RichText::new(format!("New version available: {}", latest_app)).color(egui::Color32::GREEN));
                            }
                        }

                        ui.add_space(16.0);
                        ui.heading("Veritas Version");
                        if let Some(dll_version) = self.updater.current_dll_version() {
                            ui.label(format!("Current: {}", dll_version));
                            if let Some(latest_dll) = self.updater.latest_dll_version() {
                                if latest_dll != dll_version {
                                    ui.label(RichText::new(format!("New version available: {}", latest_dll)).color(egui::Color32::GREEN));
                                }
                            }
                        } else {
                            ui.label("Not installed");
                        }

                        ui.add_space(16.0);
                        ui.heading("Source");
                        ui.vertical_centered(|ui| {
                            ui.add(Hyperlink::new("https://github.com/NightKoneko/veritas-app"));
                            ui.add(Hyperlink::new("https://github.com/hessiser/veritas"));
                        });

                        ui.add_space(16.0);
                        ui.heading("Developers");
                        ui.vertical_centered(|ui| {
                            ui.add(Hyperlink::from_label_and_url(
                                "NightKoneko",
                                "https://github.com/NightKoneko"
                            ));
                            ui.add(Hyperlink::from_label_and_url(
                                "hessiser",
                                "https://github.com/hessiser"
                            ));
                        });

                        ui.add_space(8.0);
                        if ui.button("Close").clicked() {
                            self.state.show_about = false;
                        }
                        ui.add_space(8.0);
                    });
                });
        }
    }
}
