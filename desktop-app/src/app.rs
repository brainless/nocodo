use std::sync::Arc;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DesktopApp {
    // Connection state
    connection_state: ConnectionState,

    // Configuration
    config: crate::config::DesktopConfig,

    // UI state
    show_connection_dialog: bool,
    connection_error: Option<String>,

    // Data
    projects: Vec<manager_models::Project>,

    // Runtime state (not serialized)
    #[serde(skip)]
    tunnel: Option<crate::ssh::SshTunnel>,
    #[serde(skip)]
    api_client: Option<crate::api_client::ApiClient>,
    #[serde(skip)]
    connection_result: Arc<std::sync::Mutex<Option<Result<crate::ssh::SshTunnel, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    projects_result: Arc<std::sync::Mutex<Option<Result<Vec<manager_models::Project>, String>>>>,
    #[serde(skip)]
    loading_projects: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

impl Default for DesktopApp {
    fn default() -> Self {
        Self {
            connection_state: ConnectionState::Disconnected,
            config: crate::config::DesktopConfig::default(),
            show_connection_dialog: false,
            connection_error: None,
            projects: Vec::new(),
            tunnel: None,
            api_client: None,
            connection_result: Arc::new(std::sync::Mutex::new(None)),
            projects_result: Arc::new(std::sync::Mutex::new(None)),
            loading_projects: false,
        }
    }
}

impl DesktopApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let mut app: DesktopApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        // Load configuration
        app.config = crate::config::DesktopConfig::load().unwrap_or_default();

        app
    }

    fn connect(&mut self) {
        self.connection_state = ConnectionState::Connecting;
        self.connection_error = None;
        self.connection_result = Arc::new(std::sync::Mutex::new(None));

        let server = self.config.ssh.server.clone();
        let username = self.config.ssh.username.clone();
        let key_path = if self.config.ssh.ssh_key_path.is_empty() {
            None
        } else {
            Some(self.config.ssh.ssh_key_path.clone())
        };
        let result_clone = Arc::clone(&self.connection_result);

        // Spawn async task for SSH connection
        tokio::spawn(async move {
            let result =
                crate::ssh::SshTunnel::connect(&server, &username, key_path.as_deref()).await;
            let mut connection_result = result_clone.lock().unwrap();
            *connection_result = Some(result.map_err(|e| e.to_string()));
        });
    }

    fn disconnect(&mut self) {
        self.connection_state = ConnectionState::Disconnected;

        // Disconnect SSH tunnel if it exists
        if let Some(mut tunnel) = self.tunnel.take() {
            tokio::spawn(async move {
                if let Err(e) = tunnel.disconnect().await {
                    tracing::error!("Error disconnecting SSH tunnel: {}", e);
                }
            });
        }

        self.api_client = None;
        self.projects.clear();
        self.connection_error = None;
    }

    fn refresh_projects(&mut self) {
        if self.connection_state == ConnectionState::Connected {
            if let Some(ref api_client) = self.api_client {
                self.loading_projects = true;
                self.projects_result = Arc::new(std::sync::Mutex::new(None));

                let api_client = api_client.clone();
                let result_clone = Arc::clone(&self.projects_result);

                tokio::spawn(async move {
                    let result = api_client.list_projects().await;
                    let mut projects_result = result_clone.lock().unwrap();
                    *projects_result = Some(result.map_err(|e| e.to_string()));
                });
            }
        }
    }
}

impl eframe::App for DesktopApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut should_refresh_projects = false;

        // Check for connection results
        if let Ok(mut result) = self.connection_result.try_lock() {
            if let Some(connection_result) = result.take() {
                match connection_result {
                    Ok(tunnel) => {
                        tracing::info!(
                            "SSH tunnel established successfully on port {}",
                            tunnel.local_port()
                        );
                        self.tunnel = Some(tunnel);
                        self.api_client = Some(crate::api_client::ApiClient::new(format!(
                            "http://localhost:{}",
                            self.tunnel.as_ref().unwrap().local_port()
                        )));
                        self.connection_state = ConnectionState::Connected;
                        // Mark that we should refresh projects after this block
                        should_refresh_projects = true;
                    }
                    Err(e) => {
                        tracing::error!("SSH connection failed: {}", e);
                        self.connection_error = Some(e);
                        self.connection_state = ConnectionState::Disconnected;
                    }
                }
            }
        }

        // Check for projects results
        if let Ok(mut result) = self.projects_result.try_lock() {
            if let Some(projects_result) = result.take() {
                self.loading_projects = false;
                match projects_result {
                    Ok(projects) => {
                        tracing::info!("Loaded {} projects", projects.len());
                        self.projects = projects;
                    }
                    Err(e) => {
                        tracing::error!("Failed to load projects: {}", e);
                        self.connection_error = Some(format!("Failed to load projects: {}", e));
                    }
                }
            }
        }

        // Auto-refresh projects after connection
        if should_refresh_projects {
            self.refresh_projects();
        }
        // Top panel with menu
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Connect...").clicked() {
                        self.show_connection_dialog = true;
                    }
                    if ui.button("Disconnect").clicked() {
                        self.disconnect();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("Refresh Projects").clicked() {
                        self.refresh_projects();
                    }
                });

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        // Connection status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                match &self.connection_state {
                    ConnectionState::Disconnected => {
                        ui.colored_label(egui::Color32::RED, "● Disconnected");
                    }
                    ConnectionState::Connecting => {
                        ui.colored_label(egui::Color32::YELLOW, "● Connecting...");
                    }
                    ConnectionState::Connected => {
                        ui.colored_label(egui::Color32::GREEN, "● Connected");
                        ui.label(format!("Projects: {}", self.projects.len()));
                    }
                }

                if let Some(error) = &self.connection_error {
                    ui.colored_label(egui::Color32::RED, error);
                }
            });
        });

        // Central panel with projects list
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("nocodo Projects");

            match &self.connection_state {
                ConnectionState::Disconnected => {
                    ui.vertical_centered(|ui| {
                        ui.label("Not connected to server");
                        if ui.button("Connect").clicked() {
                            self.show_connection_dialog = true;
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
                    if self.loading_projects {
                        ui.vertical_centered(|ui| {
                            ui.label("Loading projects...");
                            ui.add(egui::Spinner::new());
                        });
                    } else if self.projects.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.label("No projects found");
                            if ui.button("Refresh").clicked() {
                                self.refresh_projects();
                            }
                        });
                    } else {
                        ui.label("Projects:");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for project in &self.projects {
                                ui.horizontal(|ui| {
                                    ui.label(&project.name);
                                    ui.separator();
                                    ui.label(&project.path);
                                    if let Some(language) = &project.language {
                                        ui.separator();
                                        ui.label(language);
                                    }
                                });
                                ui.separator();
                            }
                        });
                    }
                }
            }
        });

        // Connection dialog
        if self.show_connection_dialog {
            egui::Window::new("Connect to Server")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("SSH Server:");
                    ui.text_edit_singleline(&mut self.config.ssh.server);

                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.config.ssh.username);

                    ui.label("SSH Key Path:");
                    ui.text_edit_singleline(&mut self.config.ssh.ssh_key_path);

                    ui.horizontal(|ui| {
                        if ui.button("Connect").clicked() {
                            self.connect();
                            self.show_connection_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_connection_dialog = false;
                        }
                    });
                });
        }
    }
}
