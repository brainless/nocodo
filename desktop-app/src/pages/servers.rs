use crate::state::AppState;
use crate::state::ConnectionState;
use egui::{Context, Ui};
use std::sync::Arc;

pub struct ServersPage;

impl ServersPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ServersPage {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pages::Page for ServersPage {
    fn name(&self) -> &'static str {
        "Servers"
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        ui.heading("Servers");

        // Local server section
        ui.heading("Local server:");

        ui.scope(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(6.0, 4.0);
            if ui.button("Refresh Local Server Status").clicked() {
                self.check_local_server(state);
            }
        });

        if state.ui_state.checking_local_server {
            ui.horizontal(|ui| {
                ui.label("Checking local server...");
                ui.add(egui::Spinner::new());
            });
        } else if state.ui_state.local_server_running {
            // Show grid with localhost entry
            ui.label("Local nocodo manager is running:");

            ui.scope(|ui| {
                ui.spacing_mut().button_padding = egui::vec2(6.0, 4.0);
                if ui.button("Connect Local").clicked() {
                    // Connect directly to local server without SSH via connection manager
                    state.connection_state = ConnectionState::Connecting;
                    state.ui_state.connection_error = None;
                    state.connection_result = Arc::new(std::sync::Mutex::new(None));

                    let connection_manager = Arc::clone(&state.connection_manager);
                    let result_arc = Arc::clone(&state.connection_result);

                    tokio::spawn(async move {
                        match connection_manager.connect_local(8081).await {
                            Ok(_) => {
                                tracing::info!("Connected to local manager");
                                // Store successful result
                                let mut result = result_arc.lock().unwrap();
                                *result =
                                    Some(Ok(("localhost".to_string(), "local".to_string(), 8081)));
                            }
                            Err(e) => {
                                tracing::error!("Failed to connect to local manager: {}", e);
                                // Store error result
                                let mut result = result_arc.lock().unwrap();
                                *result = Some(Err(e.to_string()));
                            }
                        }
                    });
                }
            });
        } else {
            ui.label("You can run nocodo manager locally on this computer and connect to it");
            ui.label(
                "Start the manager with: nocodo-manager --config ~/.config/nocodo/manager.toml",
            );
        }

        ui.add_space(20.0);

        // Saved servers section
        ui.heading("Saved servers:");

        if state.servers.is_empty() {
            ui.label("No servers saved");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .column(egui_extras::Column::remainder().at_least(200.0)) // Server column
                    .column(egui_extras::Column::auto()) // Port column
                    .column(egui_extras::Column::remainder().at_least(250.0)) // Key column
                    .column(egui_extras::Column::auto()) // Connect button column
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("Server");
                        });
                        header.col(|ui| {
                            ui.strong("Port");
                        });
                        header.col(|ui| {
                            ui.strong("SSH Key");
                        });
                        header.col(|ui| {
                            ui.strong("");
                        });
                    })
                    .body(|mut body| {
                        for server in &state.servers {
                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(format!("{}@{}", server.user, server.host));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{}", server.port));
                                });
                                row.col(|ui| {
                                    if let Some(key_path) = &server.key_path {
                                        ui.label(key_path);
                                    } else {
                                        ui.label(
                                            egui::RichText::new("Default")
                                                .color(ui.style().visuals.weak_text_color()),
                                        );
                                    }
                                });
                                row.col(|ui| {
                                    ui.scope(|ui| {
                                        ui.spacing_mut().button_padding = egui::vec2(6.0, 0.0);
                                        if ui.button("Connect").clicked() {
                                            state.config.ssh.server = server.host.clone();
                                            state.config.ssh.username = server.user.clone();
                                            state.config.ssh.port = server.port;
                                            state.config.ssh.ssh_key_path =
                                                server.key_path.clone().unwrap_or_default();
                                            state.ui_state.is_adding_new_server = false;
                                            state.ui_state.show_connection_dialog = true;
                                        }
                                    });
                                });
                            });
                        }
                    });
            });
        }

        ui.add_space(10.0);

        // Add New Server button
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(6.0, 4.0);
            if ui.button("+ New Server").clicked() {
                // Clear the form fields and set defaults
                state.config.ssh.server = String::new();
                state.config.ssh.username = String::new();
                state.config.ssh.port = 22;
                state.config.ssh.ssh_key_path = crate::ssh::get_default_ssh_key_path();
                state.ui_state.is_adding_new_server = true;
                state.ui_state.show_connection_dialog = true;
            }
        });
    }
}

impl ServersPage {
    fn check_local_server(&self, state: &mut AppState) {
        state.ui_state.checking_local_server = true;
        let api_service = crate::services::ApiService::new();
        let _ = api_service.check_local_server(state);
    }
}
