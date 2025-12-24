use crate::state::{AppState, ConnectionState, Server};
use egui::Context;
use egui_flex::{item, Flex, FlexAlignContent};
use std::sync::Arc;

pub struct ConnectionDialog;

impl ConnectionDialog {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConnectionDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionDialog {
    pub fn ui(&mut self, ctx: &Context, state: &mut AppState) -> bool {
        let mut should_close = false;

        if state.ui_state.show_connection_dialog {
            egui::Window::new("Connect to Server")
                .collapsible(false)
                .resizable(false)
                .fixed_size(egui::vec2(320.0, 0.0))
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::same(4))
                        .show(ui, |ui| {
                            Flex::vertical().gap(egui::vec2(0.0, 8.0)).show(ui, |flex| {
                                // SSH Server field
                                flex.add_ui(item(), |ui| {
                                    ui.label("SSH Server:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut state.config.ssh.server)
                                            .desired_width(f32::INFINITY),
                                    );
                                });

                                // Username field
                                flex.add_ui(item(), |ui| {
                                    ui.label("Username:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut state.config.ssh.username)
                                            .desired_width(f32::INFINITY),
                                    );
                                });

                                // Port field
                                flex.add_ui(item(), |ui| {
                                    ui.label("Port:");
                                    ui.add(
                                        egui::DragValue::new(&mut state.config.ssh.port)
                                            .range(1..=65535),
                                    );
                                });

                                // SSH Key Path field
                                flex.add_ui(item(), |ui| {
                                    ui.label("SSH Key Path:");
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut state.config.ssh.ssh_key_path,
                                        )
                                        .desired_width(f32::INFINITY),
                                    );
                                });

                                // Only show SSH public key when adding a new server
                                if state.ui_state.is_adding_new_server {
                                    flex.add_ui(item(), |ui| {
                                        ui.add_space(10.0);
                                    });

                                    // Display SSH public key
                                    flex.add_ui(item(), |ui| {
                                        ui.separator();
                                        ui.label(
                                            egui::RichText::new("Your SSH Public Key:").strong(),
                                        );

                                        let key_path = if state.config.ssh.ssh_key_path.is_empty() {
                                            None
                                        } else {
                                            Some(state.config.ssh.ssh_key_path.as_str())
                                        };

                                        match crate::ssh::read_ssh_public_key(key_path) {
                                            Ok(public_key) => {
                                                egui::ScrollArea::vertical().max_height(80.0).show(
                                                    ui,
                                                    |ui| {
                                                        ui.add(
                                                            egui::TextEdit::multiline(
                                                                &mut public_key.as_str(),
                                                            )
                                                            .desired_width(f32::INFINITY)
                                                            .font(egui::TextStyle::Monospace),
                                                        );
                                                    },
                                                );
                                            }
                                            Err(e) => {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "Could not read public key: {}",
                                                        e
                                                    ))
                                                    .color(ui.style().visuals.warn_fg_color),
                                                );
                                            }
                                        }
                                    });

                                    flex.add_ui(item(), |ui| {
                                        ui.add_space(10.0);
                                    });
                                }

                                // Separator and button row
                                flex.add_ui(item(), |ui| {
                                    ui.separator();
                                });

                                flex.add_ui(item(), |ui| {
                                    ui.add_space(8.0);
                                });

                                flex.add_ui(item(), |ui| {
                                    Flex::horizontal()
                                        .gap(egui::vec2(8.0, 0.0))
                                        .align_content(FlexAlignContent::End)
                                        .show(ui, |flex| {
                                            flex.add_ui(item(), |ui| {
                                                ui.scope(|ui| {
                                                    ui.spacing_mut().button_padding =
                                                        egui::vec2(6.0, 4.0);

                                                    if ui.button("Connect").clicked() {
                                                        self.connect(state);
                                                        state.ui_state.show_connection_dialog =
                                                            false;
                                                        should_close = true;
                                                    }
                                                });
                                            });

                                            flex.add_ui(item(), |ui| {
                                                ui.scope(|ui| {
                                                    ui.spacing_mut().button_padding =
                                                        egui::vec2(6.0, 4.0);

                                                    if ui.button("Cancel").clicked() {
                                                        state.ui_state.show_connection_dialog =
                                                            false;
                                                        should_close = true;
                                                    }
                                                });
                                            });
                                        });
                                });
                            });
                        });
                });
        }

        should_close
    }

    fn connect(&self, state: &mut AppState) {
        state.connection_state = ConnectionState::Connecting;
        state.ui_state.connection_error = None;
        state.connection_result = Arc::new(std::sync::Mutex::new(None));

        let server = state.config.ssh.server.clone();
        let username = state.config.ssh.username.clone();

        // Expand tilde in SSH key path
        let key_path = if state.config.ssh.ssh_key_path.is_empty() {
            None
        } else {
            let expanded_path = if state.config.ssh.ssh_key_path.starts_with("~/") {
                let home = std::env::var("HOME").unwrap_or_default();
                state.config.ssh.ssh_key_path.replacen("~", &home, 1)
            } else {
                state.config.ssh.ssh_key_path.clone()
            };
            tracing::info!("Using SSH key: {}", expanded_path);
            // Update config with expanded path
            state.config.ssh.ssh_key_path = expanded_path.clone();
            Some(expanded_path)
        };
        let remote_port = state.config.ssh.remote_port;
        let port = state.config.ssh.port;
        let connection_manager = Arc::clone(&state.connection_manager);
        let result_arc = Arc::clone(&state.connection_result);

        // Get a flag to signal when to refresh servers list
        let servers_refresh_needed = Arc::new(std::sync::Mutex::new(false));
        let servers_refresh_flag = Arc::clone(&servers_refresh_needed);

        tracing::info!(
            "Initiating SSH connection to {}@{}:{}",
            username,
            server,
            port
        );

        // Spawn async task for SSH connection via connection manager
        tokio::spawn(async move {
            tracing::info!("Starting SSH tunnel connection...");
            match connection_manager
                .connect_ssh(&server, &username, key_path.as_deref(), port, remote_port)
                .await
            {
                Ok(_) => {
                    tracing::info!("SSH tunnel established successfully");

                    // Save server to database if not already saved
                    // We need to do this here because SSH has successfully connected
                    // Get the database path
                    if let Some(config_dir) = dirs::config_dir() {
                        let nocodo_dir = config_dir.join("nocodo");
                        let db_path = nocodo_dir.join("local.sqlite3");

                        if let Ok(db) = rusqlite::Connection::open(&db_path) {
                            let new_server = Server {
                                host: server.clone(),
                                user: username.clone(),
                                key_path: key_path.clone(),
                                port,
                            };

                            // Check if this server already exists
                            let exists: Result<i64, _> = db.query_row(
                                "SELECT COUNT(*) FROM servers WHERE host = ?1 AND user = ?2 AND COALESCE(key_path, '') = COALESCE(?3, '') AND port = ?4",
                                rusqlite::params![&new_server.host, &new_server.user, &new_server.key_path, new_server.port],
                                |row| row.get(0),
                            );

                            if let Ok(count) = exists {
                                if count == 0 {
                                    // Insert the new server
                                    if db.execute(
                                        "INSERT INTO servers (host, user, key_path, port) VALUES (?1, ?2, ?3, ?4)",
                                        rusqlite::params![&new_server.host, &new_server.user, &new_server.key_path, new_server.port],
                                    ).is_ok() {
                                        tracing::info!("Saved new server to database: {}@{}", username, server);
                                        // Signal that servers list needs refresh
                                        if let Ok(mut flag) = servers_refresh_flag.lock() {
                                            *flag = true;
                                        }
                                    } else {
                                        tracing::warn!("Failed to save server to database");
                                    }
                                } else {
                                    tracing::debug!("Server already exists in database");
                                }
                            }
                        }
                    }

                    // Wait a moment for the tunnel to be fully ready
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                    // Verify we can reach the API through the tunnel
                    if let Some(api_client_arc) = connection_manager.get_api_client().await {
                        let api_client = api_client_arc.read().await;
                        tracing::info!("API client created, testing connection...");
                        match api_client.health_check().await {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully connected to nocodo manager at {}@{}",
                                    username,
                                    server
                                );
                                let mut result = result_arc.lock().unwrap();
                                *result = Some(Ok((server.clone(), username.clone(), port)));
                            }
                            Err(e) => {
                                tracing::error!("Failed to reach API through tunnel: {}", e);
                                let mut result = result_arc.lock().unwrap();
                                *result = Some(Err(format!("SSH tunnel OK but cannot reach nocodo manager: {}. Is nocodo-manager running on the server?", e)));
                            }
                        }
                    } else {
                        tracing::error!("Failed to get API client after SSH tunnel");
                        let mut result = result_arc.lock().unwrap();
                        *result = Some(Err(
                            "SSH tunnel established but API client not available".to_string()
                        ));
                    }
                }
                Err(e) => {
                    tracing::error!("SSH connection failed: {}", e);
                    let mut result = result_arc.lock().unwrap();
                    *result = Some(Err(format!("SSH connection failed: {}", e)));
                }
            }
        });

        // Store the refresh flag in the state so we can check it in the UI loop
        state.ui_state.servers_refresh_needed = servers_refresh_needed;
    }
}
