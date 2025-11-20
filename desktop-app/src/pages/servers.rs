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

            let card_width = 300.0;
            let card_height = 60.0;

            ui.horizontal_wrapped(|ui| {
                let response = ui.allocate_ui(egui::vec2(card_width, card_height), |ui| {
                    egui::Frame::NONE
                        .fill(ui.style().visuals.widgets.inactive.bg_fill)
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("localhost").size(14.0).strong());
                                    ui.label(
                                        egui::RichText::new("No key required")
                                            .size(12.0)
                                            .color(ui.style().visuals.weak_text_color()),
                                    );
                                });
                            });
                        });
                });

                // Make the card clickable
                if response.response.interact(egui::Sense::click()).clicked() {
                    // Connect directly to local server without SSH via connection manager
                    let connection_manager = Arc::clone(&state.connection_manager);
                    tokio::spawn(async move {
                        match connection_manager.connect_local(8081).await {
                            Ok(_) => tracing::info!("Connected to local manager"),
                            Err(e) => tracing::error!("Failed to connect to local manager: {}", e),
                        }
                    });

                    state.connection_state = ConnectionState::Connected;
                    state.ui_state.connected_host = Some("localhost".to_string());
                    state.models_fetch_attempted = false; // Reset to allow fetching models on new connection
                                                          // Refresh data after connecting
                    self.refresh_projects(state);
                    self.refresh_works(state);
                    self.refresh_settings(state);
                }

                // Change cursor to pointer on hover
                if response.response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
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

    fn refresh_projects(&self, state: &mut AppState) {
        let api_service = crate::services::ApiService::new();
        api_service.refresh_projects(state);
    }

    fn refresh_works(&self, state: &mut AppState) {
        let api_service = crate::services::ApiService::new();
        api_service.refresh_works(state);
    }

    fn refresh_settings(&self, state: &mut AppState) {
        let api_service = crate::services::ApiService::new();
        api_service.refresh_supported_models(state);
    }
}
