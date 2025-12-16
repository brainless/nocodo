use crate::state::AppState;
use egui::Context;
use egui_flex::{item, Flex, FlexAlignContent};
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
                .fixed_size(egui::vec2(320.0, 0.0))
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::same(4))
                        .show(ui, |ui| {
                            Flex::vertical()
                                .gap(egui::vec2(0.0, 8.0))
                                .show(ui, |flex| {
                                    // Show error message if any
                                    if let Some(ref error) = self.error_message {
                                        flex.add_ui(item(), |ui| {
                                            ui.colored_label(egui::Color32::RED, error);
                                        });
                                        flex.add_ui(item(), |ui| {
                                            ui.add_space(4.0);
                                        });
                                    }

                                    // Username field
                                    flex.add_ui(item(), |ui| {
                                        ui.label("Username:");
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.username)
                                                .desired_width(f32::INFINITY)
                                        );
                                    });

                                    // Email field (only in register mode)
                                    if self.is_register_mode {
                                        flex.add_ui(item(), |ui| {
                                            ui.label("Email (optional):");
                                            ui.add(
                                                egui::TextEdit::singleline(&mut self.email)
                                                    .desired_width(f32::INFINITY)
                                            );
                                        });
                                    }

                                    // Password field
                                    flex.add_ui(item(), |ui| {
                                        ui.label("Password:");
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.password)
                                                .password(true)
                                                .desired_width(f32::INFINITY)
                                        );
                                    });

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
                                                        ui.spacing_mut().button_padding = egui::vec2(6.0, 4.0);

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
                                                    });
                                                });

                                                flex.add_ui(item(), |ui| {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().button_padding = egui::vec2(6.0, 4.0);

                                                        if ui.button("Cancel").clicked() {
                                                            state.ui_state.show_auth_dialog = false;
                                                            should_close = true;
                                                        }
                                                    });
                                                });
                                            });
                                    });

                                    flex.add_ui(item(), |ui| {
                                        ui.add_space(10.0);
                                    });

                                    // Toggle between login and register
                                    flex.add_ui(item(), |ui| {
                                        Flex::horizontal()
                                            .gap(egui::vec2(4.0, 0.0))
                                            .align_content(FlexAlignContent::Center)
                                            .show(ui, |flex| {
                                                if self.is_register_mode {
                                                    flex.add_ui(item(), |ui| {
                                                        ui.label("Already have an account?");
                                                    });
                                                    flex.add_ui(item(), |ui| {
                                                        if ui.link("Login").clicked() {
                                                            self.is_register_mode = false;
                                                            self.error_message = None;
                                                        }
                                                    });
                                                } else {
                                                    flex.add_ui(item(), |ui| {
                                                        ui.label("Don't have an account?");
                                                    });
                                                    flex.add_ui(item(), |ui| {
                                                        if ui.link("Register").clicked() {
                                                            self.is_register_mode = true;
                                                            self.error_message = None;
                                                        }
                                                    });
                                                }
                                            });
                                    });
                                });
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

        // Determine fingerprint based on connection type
        // For local connections, use machine fingerprint; for SSH, use SSH key fingerprint
        let is_local_connection = state.ui_state.connected_host.as_deref() == Some("localhost");

        let ssh_fingerprint = if is_local_connection {
            // Local connection - use machine-specific fingerprint
            match crate::ssh::generate_local_fingerprint() {
                Ok(fingerprint) => fingerprint,
                Err(e) => {
                    self.error_message = Some(format!("Failed to generate machine fingerprint: {}", e));
                    return;
                }
            }
        } else {
            // SSH connection - use SSH key fingerprint
            match crate::ssh::calculate_ssh_fingerprint(
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
        let login_result = Arc::clone(&state.login_result);

        // Set flag to navigate after successful authentication
        state.ui_state.should_navigate_after_auth = true;

        // Spawn async task for login
        tokio::spawn(async move {
            match connection_manager
                .login(&username, &password, &ssh_fingerprint)
                .await
            {
                Ok(login_response) => {
                    tracing::info!(
                        "Login successful for user: {}",
                        login_response.user.username
                    );

                    // Store login result for state update
                    {
                        let mut login_result_lock = login_result.lock().unwrap();
                        *login_result_lock = Some(Ok(login_response));
                    }

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
                            let mut supported_models_result_lock =
                                supported_models_result.lock().unwrap();
                            *supported_models_result_lock = Some(result.map_err(|e| e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Login failed: {}", e);
                    let mut login_result_lock = login_result.lock().unwrap();
                    *login_result_lock = Some(Err(e.to_string()));
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

        // Determine fingerprint and public key based on connection type
        let is_local_connection = state.ui_state.connected_host.as_deref() == Some("localhost");

        let (ssh_fingerprint, ssh_public_key) = if is_local_connection {
            // Local connection - use machine fingerprint and placeholder public key
            let fingerprint = match crate::ssh::generate_local_fingerprint() {
                Ok(fp) => fp,
                Err(e) => {
                    self.error_message = Some(format!("Failed to generate machine fingerprint: {}", e));
                    return;
                }
            };
            // Use fingerprint as the public key for local connections
            (fingerprint.clone(), fingerprint)
        } else {
            // SSH connection - use SSH key fingerprint and public key
            let fingerprint = match crate::ssh::calculate_ssh_fingerprint(
                if state.config.ssh.ssh_key_path.is_empty() {
                    None
                } else {
                    Some(&state.config.ssh.ssh_key_path)
                },
            ) {
                Ok(fp) => fp,
                Err(e) => {
                    self.error_message = Some(format!("Failed to calculate SSH fingerprint: {}", e));
                    return;
                }
            };

            let public_key = match crate::ssh::read_ssh_public_key(
                if state.config.ssh.ssh_key_path.is_empty() {
                    None
                } else {
                    Some(&state.config.ssh.ssh_key_path)
                },
            ) {
                Ok(pk) => pk,
                Err(e) => {
                    self.error_message = Some(format!("Failed to read SSH public key: {}", e));
                    return;
                }
            };

            (fingerprint, public_key)
        };

        let username = self.username.clone();
        let password = self.password.clone();
        let email = if self.email.trim().is_empty() {
            None
        } else {
            Some(self.email.clone())
        };
        let connection_manager = Arc::clone(&state.connection_manager);
        let login_result = Arc::clone(&state.login_result);
        let projects_result = Arc::clone(&state.projects_result);
        let works_result = Arc::clone(&state.works_result);
        let settings_result = Arc::clone(&state.settings_result);
        let supported_models_result = Arc::clone(&state.supported_models_result);

        // Set flag to navigate after successful authentication
        state.ui_state.should_navigate_after_auth = true;

        // Spawn async task for registration
        tokio::spawn(async move {
            match connection_manager
                .register(
                    &username,
                    &password,
                    email.as_deref(),
                    &ssh_public_key,
                    &ssh_fingerprint,
                )
                .await
            {
                Ok(user_response) => {
                    tracing::info!(
                        "Registration successful for user: {}",
                        user_response.user.name
                    );
                    // After registration, automatically log in
                    match connection_manager
                        .login(&username, &password, &ssh_fingerprint)
                        .await
                    {
                        Ok(login_response) => {
                            tracing::info!("Auto-login successful after registration");
                            // Store login result for state update
                            {
                                let mut login_result_lock = login_result.lock().unwrap();
                                *login_result_lock = Some(Ok(login_response));
                            }

                            // Refresh all data after successful login
                            if let Some(api_client_arc) = connection_manager.get_api_client().await {
                                let api_client = api_client_arc.read().await;

                                let result = api_client.list_projects().await;
                                {
                                    let mut projects_result_lock = projects_result.lock().unwrap();
                                    *projects_result_lock = Some(result.map_err(|e| e.to_string()));
                                }

                                let result = api_client.list_works().await;
                                {
                                    let mut works_result_lock = works_result.lock().unwrap();
                                    *works_result_lock = Some(result.map_err(|e| e.to_string()));
                                }

                                let result = api_client.get_settings().await;
                                {
                                    let mut settings_result_lock = settings_result.lock().unwrap();
                                    *settings_result_lock = Some(result.map_err(|e| e.to_string()));
                                }

                                let result = api_client.get_supported_models().await;
                                {
                                    let mut supported_models_result_lock =
                                        supported_models_result.lock().unwrap();
                                    *supported_models_result_lock = Some(result.map_err(|e| e.to_string()));
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Auto-login after registration failed: {}", e);
                            let mut login_result_lock = login_result.lock().unwrap();
                            *login_result_lock = Some(Err(e.to_string()));
                        }
                    }
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
