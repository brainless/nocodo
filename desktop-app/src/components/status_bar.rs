use crate::state::AppState;
use crate::state::ConnectionState;
use egui::Context;

pub struct StatusBar;

impl StatusBar {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBar {
    pub fn ui(&mut self, ctx: &Context, state: &AppState) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                match &state.connection_state {
                    ConnectionState::Disconnected => {
                        ui.colored_label(egui::Color32::RED, "● Disconnected");
                    }
                    ConnectionState::Connecting => {
                        ui.colored_label(egui::Color32::YELLOW, "● Connecting...");
                    }
                    ConnectionState::Connected => {
                        let label = if let Some(host) = &state.ui_state.connected_host {
                            format!("● Connected: {}", host)
                        } else {
                            "● Connected".to_string()
                        };
                        ui.colored_label(egui::Color32::GREEN, label);
                        ui.label(format!("Projects: {}", state.projects.len()));
                    }
                    ConnectionState::Error(error) => {
                        ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                    }
                }

                if let Some(error) = &state.ui_state.connection_error {
                    ui.colored_label(egui::Color32::RED, error);
                }
            });
        });
    }
}
