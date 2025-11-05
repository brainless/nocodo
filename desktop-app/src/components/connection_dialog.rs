use crate::state::{AppState, ConnectionState, Server};
use egui::Context;
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
                .show(ctx, |ui| {
                    ui.label("SSH Server:");
                    ui.text_edit_singleline(&mut state.config.ssh.server);

                    ui.label("Username:");
                    ui.text_edit_singleline(&mut state.config.ssh.username);

                    ui.label("Port:");
                    ui.add(egui::DragValue::new(&mut state.config.ssh.port).range(1..=65535));

                    ui.label("SSH Key Path:");
                    ui.text_edit_singleline(&mut state.config.ssh.ssh_key_path);

                    ui.horizontal(|ui| {
                        if ui.button("Connect").clicked() {
                            self.connect(state);
                            state.ui_state.show_connection_dialog = false;
                            should_close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            state.ui_state.show_connection_dialog = false;
                            should_close = true;
                        }
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
                                *result = Some(Ok(server.clone()));
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
