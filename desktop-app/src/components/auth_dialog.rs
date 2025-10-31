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

        // Spawn async task for login
        tokio::spawn(async move {
            match connection_manager.login(&username, &password, &ssh_fingerprint).await {
                Ok(login_response) => {
                    tracing::info!("Login successful for user: {}", login_response.user.username);
                    // Connection manager will handle setting the token on ApiClient
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
            match connection_manager.register(&username, &password, email.as_deref()).await {
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
