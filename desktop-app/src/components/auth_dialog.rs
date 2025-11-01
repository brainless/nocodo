use crate::state::AppState;
use egui::Context;
use std::sync::Arc;

pub struct AuthDialog {
    username: String,
    password: String,
    email: String,
    is_register_mode: bool,
    error_message: Option<String>,
}

impl AuthDialog {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            email: String::new(),
            is_register_mode: false,
            error_message: None,
        }
    }
}

impl Default for AuthDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthDialog {
    pub fn ui(&mut self, ctx: &Context, state: &mut AppState) -> bool {
        let mut should_close = false;

        if state.ui_state.show_auth_dialog {
            let title = if self.is_register_mode {
                "Register"
            } else {
                "Login"
            };

            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    // Show error message if any
                    if let Some(ref error) = self.error_message {
                        ui.colored_label(egui::Color32::RED, error);
                    }

                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);

                    if self.is_register_mode {
                        ui.label("Email (optional):");
                        ui.text_edit_singleline(&mut self.email);
                    }

                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        let button_text = if self.is_register_mode {
                            "Register"
                        } else {
                            "Login"
                        };

                        if ui.button(button_text).clicked() {
                            if self.is_register_mode {
                                self.register(state);
                            } else {
                                self.login(state);
                            }
                        }

                        if ui.button("Cancel").clicked() {
                            state.ui_state.show_auth_dialog = false;
                            should_close = true;
                        }
                    });

                    ui.add_space(10.0);

                    // Toggle between login and register
                    ui.horizontal(|ui| {
                        if self.is_register_mode {
                            ui.label("Already have an account?");
                            if ui.link("Login").clicked() {
                                self.is_register_mode = false;
                                self.error_message = None;
                            }
                        } else {
                            ui.label("Don't have an account?");
                            if ui.link("Register").clicked() {
                                self.is_register_mode = true;
                                self.error_message = None;
                            }
                        }
                    });
                });
        }

        should_close
    }

    fn login(&mut self, state: &mut AppState) {
        // Validate input
        if self.username.trim().is_empty() || self.password.trim().is_empty() {
            self.error_message = Some("Username and password are required".to_string());
            return;
        }

        // Calculate SSH fingerprint
        let ssh_fingerprint = match crate::ssh::calculate_ssh_fingerprint(
            if state.config.ssh.ssh_key_path.is_empty() {
                None
            } else {
                Some(&state.config.ssh.ssh_key_path)
            },
        ) {
            Ok(fingerprint) => fingerprint,
            Err(e) => {
                self.error_message = Some(format!("Failed to calculate SSH fingerprint: {}", e));
                return;
            }
        };

        let username = self.username.clone();
        let password = self.password.clone();
        let connection_manager = Arc::clone(&state.connection_manager);

        // Clone state fields needed for refresh
        let projects_result = Arc::clone(&state.projects_result);
        let works_result = Arc::clone(&state.works_result);
        let settings_result = Arc::clone(&state.settings_result);
        let supported_models_result = Arc::clone(&state.supported_models_result);

        // Spawn async task for login
        tokio::spawn(async move {
            match connection_manager.login(&username, &password, &ssh_fingerprint).await {
                Ok(login_response) => {
                    tracing::info!("Login successful for user: {}", login_response.user.username);

                    // Refresh all data after successful login
                    if let Some(api_client_arc) = connection_manager.get_api_client().await {
                        let api_client = api_client_arc.read().await;

                        // Refresh projects
                        let result = api_client.list_projects().await;
                        {
                            let mut projects_result_lock = projects_result.lock().unwrap();
                            *projects_result_lock = Some(result.map_err(|e| e.to_string()));
                        }

                        // Refresh works
                        let result = api_client.list_works().await;
                        {
                            let mut works_result_lock = works_result.lock().unwrap();
                            *works_result_lock = Some(result.map_err(|e| e.to_string()));
                        }

                        // Refresh settings
                        let result = api_client.get_settings().await;
                        {
                            let mut settings_result_lock = settings_result.lock().unwrap();
                            *settings_result_lock = Some(result.map_err(|e| e.to_string()));
                        }

                        // Refresh supported models
                        let result = api_client.get_supported_models().await;
                        {
                            let mut supported_models_result_lock = supported_models_result.lock().unwrap();
                            *supported_models_result_lock = Some(result.map_err(|e| e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Login failed: {}", e);
                }
            }
        });

        // Close dialog (result will be handled by connection manager)
        state.ui_state.show_auth_dialog = false;
    }

    fn register(&mut self, state: &mut AppState) {
        // Validate input
        if self.username.trim().is_empty() || self.password.trim().is_empty() {
            self.error_message = Some("Username and password are required".to_string());
            return;
        }

        // Calculate SSH fingerprint and read public key
        let ssh_fingerprint = match crate::ssh::calculate_ssh_fingerprint(
            if state.config.ssh.ssh_key_path.is_empty() {
                None
            } else {
                Some(&state.config.ssh.ssh_key_path)
            },
        ) {
            Ok(fingerprint) => fingerprint,
            Err(e) => {
                self.error_message = Some(format!("Failed to calculate SSH fingerprint: {}", e));
                return;
            }
        };

        let ssh_public_key = match crate::ssh::read_ssh_public_key(
            if state.config.ssh.ssh_key_path.is_empty() {
                None
            } else {
                Some(&state.config.ssh.ssh_key_path)
            },
        ) {
            Ok(public_key) => public_key,
            Err(e) => {
                self.error_message = Some(format!("Failed to read SSH public key: {}", e));
                return;
            }
        };

        let username = self.username.clone();
        let password = self.password.clone();
        let email = if self.email.trim().is_empty() {
            None
        } else {
            Some(self.email.clone())
        };
        let connection_manager = Arc::clone(&state.connection_manager);

        // Spawn async task for registration
        tokio::spawn(async move {
            match connection_manager.register(&username, &password, email.as_deref(), &ssh_public_key, &ssh_fingerprint).await {
                Ok(user_response) => {
                    tracing::info!("Registration successful for user: {}", user_response.user.username);
                    // After registration, automatically log in
                }
                Err(e) => {
                    tracing::error!("Registration failed: {}", e);
                }
            }
        });

        // Close dialog (result will be handled by connection manager)
        state.ui_state.show_auth_dialog = false;
    }
}
