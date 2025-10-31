use crate::state::{AppState, ConnectionState};
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

                    // Wait a moment for the tunnel to be fully ready
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                    // Verify we can reach the API through the tunnel
                    if let Some(api_client) = connection_manager.get_api_client().await {
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
    }
}
