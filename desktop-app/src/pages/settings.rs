use crate::pages::Page;
use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use crate::state::ConnectionState;
use egui::{Context, Ui};

pub struct SettingsPage;

impl SettingsPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pages::Page for SettingsPage {
    fn name(&self) -> &'static str {
        "Settings"
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        ui.heading("Settings");

        match &state.connection_state {
            ConnectionState::Disconnected => {
                ui.vertical_centered(|ui| {
                    ui.label("Not connected to server");
                    if ui.button("Connect").clicked() {
                        state.ui_state.show_connection_dialog = true;
                    }
                });
            }
            ConnectionState::Connecting => {
                ui.vertical_centered(|ui| {
                    ui.label("Connecting...");
                    ui.add(egui::Spinner::new());
                });
            }
            ConnectionState::Connected => {
                if state.loading_settings {
                    ui.vertical_centered(|ui| {
                        ui.label("Loading settings...");
                        ui.add(egui::Spinner::new());
                    });
                } else if let Some(settings) = &state.settings {
                    ui.heading("API Keys");

                    // Clone settings to avoid borrowing issues
                    let settings_clone = settings.clone();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Grok/xAI API Key
                        ui.vertical(|ui| {
                            ui.label("Grok API Key");
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut state.xai_api_key_input)
                                    .password(true)
                                    .hint_text("Enter xAI API key")
                                    .desired_width(400.0),
                            );
                            if response.changed() {
                                state.api_keys_modified = true;
                            }
                            ui.horizontal(|ui| {
                                let configured = settings_clone
                                    .api_keys
                                    .iter()
                                    .find(|k| k.name == "Grok API Key")
                                    .map(|k| k.is_configured)
                                    .unwrap_or(false);
                                if configured {
                                    ui.colored_label(egui::Color32::GREEN, "✅ Configured");
                                } else {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(255, 165, 0),
                                        "❌ Not configured",
                                    );
                                }
                            });
                        });
                        ui.separator();

                        // OpenAI API Key
                        ui.vertical(|ui| {
                            ui.label("OpenAI API Key");
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut state.openai_api_key_input)
                                    .password(true)
                                    .hint_text("Enter OpenAI API key")
                                    .desired_width(400.0),
                            );
                            if response.changed() {
                                state.api_keys_modified = true;
                            }
                            ui.horizontal(|ui| {
                                let configured = settings_clone
                                    .api_keys
                                    .iter()
                                    .find(|k| k.name == "OpenAI API Key")
                                    .map(|k| k.is_configured)
                                    .unwrap_or(false);
                                if configured {
                                    ui.colored_label(egui::Color32::GREEN, "✅ Configured");
                                } else {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(255, 165, 0),
                                        "❌ Not configured",
                                    );
                                }
                            });
                        });
                        ui.separator();

                        // Anthropic API Key
                        ui.vertical(|ui| {
                            ui.label("Anthropic API Key");
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut state.anthropic_api_key_input)
                                    .password(true)
                                    .hint_text("Enter Anthropic API key")
                                    .desired_width(400.0),
                            );
                            if response.changed() {
                                state.api_keys_modified = true;
                            }
                            ui.horizontal(|ui| {
                                let configured = settings_clone
                                    .api_keys
                                    .iter()
                                    .find(|k| k.name == "Anthropic API Key")
                                    .map(|k| k.is_configured)
                                    .unwrap_or(false);
                                if configured {
                                    ui.colored_label(egui::Color32::GREEN, "✅ Configured");
                                } else {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(255, 165, 0),
                                        "❌ Not configured",
                                    );
                                }
                            });
                        });
                        ui.separator();

                        // Update Keys button
                        ui.horizontal(|ui| {
                            let update_button = ui.add_enabled(
                                state.api_keys_modified && !state.updating_api_keys,
                                egui::Button::new("Update API Keys"),
                            );

                            if update_button.clicked() {
                                self.update_api_keys(state);
                            }

                            if state.updating_api_keys {
                                ui.add(egui::Spinner::new());
                            }
                        });
                    });

                    ui.add_space(20.0);
                    ui.heading("Projects Default Path");

                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        let response =
                            ui.text_edit_singleline(&mut state.ui_state.projects_default_path);
                        if response.changed() {
                            state.projects_default_path_modified = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        let update_button = ui.add_enabled(
                            state.projects_default_path_modified && !state.updating_projects_path,
                            egui::Button::new("Update path"),
                        );

                        if update_button.clicked() {
                            self.update_projects_default_path(state);
                        }

                        if state.updating_projects_path {
                            ui.add(egui::Spinner::new());
                        }

                        ui.add_space(10.0);

                        let scan_button = ui.add_enabled(
                            !state.scanning_projects,
                            egui::Button::new("Scan and load projects"),
                        );

                        if scan_button.clicked() {
                            self.scan_projects(state);
                        }

                        if state.scanning_projects {
                            ui.add(egui::Spinner::new());
                        }
                    });

                    if ui.button("Refresh Settings").clicked() {
                        self.refresh_settings(state);
                    }

                    ui.add_space(30.0);
                    ui.heading("UI Reference");

                    if ui.button("2 column main content").clicked() {
                        state.ui_state.current_page = UiPage::UiTwoColumnMainContent;
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.label("No settings loaded");
                        if ui.button("Refresh").clicked() {
                            self.refresh_settings(state);
                        }
                    });
                }
            }
            ConnectionState::Error(error) => {
                ui.vertical_centered(|ui| {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                    if ui.button("Retry").clicked() {
                        state.ui_state.show_connection_dialog = true;
                    }
                });
            }
        }
    }
}

impl SettingsPage {
    fn update_api_keys(&self, state: &mut AppState) {
        state.updating_api_keys = true;
        // This will be implemented when we extract the API methods
    }

    fn update_projects_default_path(&self, state: &mut AppState) {
        state.updating_projects_path = true;
        // This will be implemented when we extract the API methods
    }

    fn scan_projects(&self, state: &mut AppState) {
        state.scanning_projects = true;
        // This will be implemented when we extract the API methods
    }

    fn refresh_settings(&self, state: &mut AppState) {
        state.loading_settings = true;
        // This will be implemented when we extract the API methods
    }
}
