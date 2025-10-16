use rusqlite::Connection;
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
    show_new_work_dialog: bool,
    new_work_title: String,
    new_work_project_id: Option<i64>,
    new_work_model: String,
    connection_error: Option<String>,
    connected_host: Option<String>,

    // Data
    projects: Vec<manager_models::Project>,
    works: Vec<manager_models::Work>,
    work_messages: Vec<manager_models::WorkMessage>,
    ai_session_outputs: Vec<manager_models::AiSessionOutput>,
    servers: Vec<Server>,
    current_page: Page,

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
    #[allow(clippy::type_complexity)]
    works_result: Arc<std::sync::Mutex<Option<Result<Vec<manager_models::Work>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    work_messages_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::WorkMessage>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    ai_session_outputs_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::AiSessionOutput>, String>>>>,
    #[serde(skip)]
    loading_projects: bool,
    #[serde(skip)]
    loading_works: bool,
    #[serde(skip)]
    loading_work_messages: bool,
    #[serde(skip)]
    loading_ai_session_outputs: bool,
    #[serde(skip)]
    creating_work: bool,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    create_work_result: Arc<std::sync::Mutex<Option<Result<manager_models::Work, String>>>>,
    #[serde(skip)]
    db: Option<Connection>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
enum Page {
    Projects,
    Work,
    WorkDetail(i64), // Work ID
    Mentions,
    Servers,
    Settings,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Server {
    host: String,
    user: String,
    key_path: Option<String>,
}

impl Default for DesktopApp {
    fn default() -> Self {
        Self {
            connection_state: ConnectionState::Disconnected,
            config: crate::config::DesktopConfig::default(),
            show_connection_dialog: false,
            show_new_work_dialog: false,
            new_work_title: String::new(),
            new_work_project_id: None,
            new_work_model: String::new(),
            connection_error: None,
            connected_host: None,
            projects: Vec::new(),
            works: Vec::new(),
            work_messages: Vec::new(),
            ai_session_outputs: Vec::new(),
            servers: Vec::new(),
            current_page: Page::Projects,
            tunnel: None,
            api_client: None,
            connection_result: Arc::new(std::sync::Mutex::new(None)),
            projects_result: Arc::new(std::sync::Mutex::new(None)),
            works_result: Arc::new(std::sync::Mutex::new(None)),
            work_messages_result: Arc::new(std::sync::Mutex::new(None)),
            ai_session_outputs_result: Arc::new(std::sync::Mutex::new(None)),
            loading_projects: false,
            loading_works: false,
            loading_work_messages: false,
            loading_ai_session_outputs: false,
            creating_work: false,
            create_work_result: Arc::new(std::sync::Mutex::new(None)),
            db: None,
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

        // Initialize local database
        let config_dir = dirs::config_dir().expect("Could not find config dir");
        let nocodo_dir = config_dir.join("nocodo");
        std::fs::create_dir_all(&nocodo_dir).expect("Could not create nocodo config dir");
        let db_path = nocodo_dir.join("local.sqlite3");
        let db = Connection::open(&db_path).expect("Could not open DB");
        db.execute(
            "CREATE TABLE IF NOT EXISTS servers (
                id INTEGER PRIMARY KEY,
                host TEXT NOT NULL,
                user TEXT NOT NULL,
                key_path TEXT
            )",
            [],
        )
        .expect("Could not create servers table");
        db.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_servers_unique ON servers (host, user, key_path)",
            [],
        ).expect("Could not create unique index");

        // Load existing servers
        {
            let mut stmt = db
                .prepare("SELECT host, user, key_path FROM servers")
                .expect("Could not prepare statement");
            let server_iter = stmt
                .query_map([], |row| {
                    Ok(Server {
                        host: row.get(0)?,
                        user: row.get(1)?,
                        key_path: row.get(2)?,
                    })
                })
                .expect("Could not query servers");
            app.servers = server_iter.filter_map(|s| s.ok()).collect();
        }

        app.db = Some(db);

        // Always start disconnected - never restore connection state
        app.connection_state = ConnectionState::Disconnected;
        app.tunnel = None;
        app.api_client = None;
        app.projects.clear();
        app.works.clear();
        app.work_messages.clear();
        app.ai_session_outputs.clear();
        app.connection_error = None;
        app.loading_projects = false;
        app.loading_works = false;
        app.loading_work_messages = false;
        app.loading_ai_session_outputs = false;

        app
    }

    fn connect(&mut self) {
        self.connection_state = ConnectionState::Connecting;
        self.connection_error = None;
        self.connection_result = Arc::new(std::sync::Mutex::new(None));

        let server = self.config.ssh.server.clone();
        let username = self.config.ssh.username.clone();

        // Expand tilde in SSH key path
        let key_path = if self.config.ssh.ssh_key_path.is_empty() {
            None
        } else {
            let expanded_path = if self.config.ssh.ssh_key_path.starts_with("~/") {
                let home = std::env::var("HOME").unwrap_or_default();
                self.config.ssh.ssh_key_path.replacen("~", &home, 1)
            } else {
                self.config.ssh.ssh_key_path.clone()
            };
            tracing::info!("Using SSH key: {}", expanded_path);
            // Update config with expanded path
            self.config.ssh.ssh_key_path = expanded_path.clone();
            Some(expanded_path)
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

    fn refresh_works(&mut self) {
        if self.connection_state == ConnectionState::Connected {
            if let Some(ref api_client) = self.api_client {
                self.loading_works = true;
                self.works_result = Arc::new(std::sync::Mutex::new(None));

                let api_client = api_client.clone();
                let result_clone = Arc::clone(&self.works_result);

                tokio::spawn(async move {
                    let result = api_client.list_works().await;
                    let mut works_result = result_clone.lock().unwrap();
                    *works_result = Some(result.map_err(|e| e.to_string()));
                });
            }
        }
    }

    fn refresh_work_messages(&mut self, work_id: i64) {
        if self.connection_state == ConnectionState::Connected {
            if let Some(ref api_client) = self.api_client {
                // Fetch both work messages and AI session outputs
                self.loading_work_messages = true;
                self.loading_ai_session_outputs = true;
                self.work_messages_result = Arc::new(std::sync::Mutex::new(None));
                self.ai_session_outputs_result = Arc::new(std::sync::Mutex::new(None));

                let api_client_clone1 = api_client.clone();
                let api_client_clone2 = api_client.clone();
                let messages_result_clone = Arc::clone(&self.work_messages_result);
                let outputs_result_clone = Arc::clone(&self.ai_session_outputs_result);

                // Fetch work messages (user input)
                tokio::spawn(async move {
                    let result = api_client_clone1.get_work_messages(work_id).await;
                    let mut work_messages_result = messages_result_clone.lock().unwrap();
                    *work_messages_result = Some(result.map_err(|e| e.to_string()));
                });

                // Fetch AI session outputs (AI responses and tool results)
                tokio::spawn(async move {
                    let result = api_client_clone2.get_ai_session_outputs(work_id).await;
                    let mut ai_session_outputs_result = outputs_result_clone.lock().unwrap();
                    *ai_session_outputs_result = Some(result.map_err(|e| e.to_string()));
                });
            }
        }
    }

    fn create_work(&mut self) {
        if self.connection_state == ConnectionState::Connected {
            if let Some(ref api_client) = self.api_client {
                self.creating_work = true;
                self.create_work_result = Arc::new(std::sync::Mutex::new(None));

                let api_client = api_client.clone();
                let result_clone = Arc::clone(&self.create_work_result);

                let title = self.new_work_title.clone();
                let project_id = self.new_work_project_id;
                let model = if self.new_work_model.is_empty() {
                    None
                } else {
                    Some(self.new_work_model.clone())
                };

                tokio::spawn(async move {
                    let request = manager_models::CreateWorkRequest {
                        title,
                        project_id,
                        model,
                    };
                    let result = api_client.create_work(request).await;
                    let mut create_work_result = result_clone.lock().unwrap();
                    *create_work_result = Some(result.map_err(|e| e.to_string()));
                });
            }
        }
    }

    /// Helper function to create a sidebar link with proper styling
    fn sidebar_link(
        &self,
        ui: &mut egui::Ui,
        text: &str,
        default_bg: egui::Color32,
        hover_bg: egui::Color32,
    ) -> bool {
        let available_width = ui.available_width();
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(available_width, 24.0), egui::Sense::click());

        // Change cursor to pointer on hover
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Determine background color based on hover state
        let bg_color = if response.hovered() {
            hover_bg
        } else {
            default_bg
        };

        // Draw background
        ui.painter().rect_filled(rect, 0.0, bg_color);

        // Draw text (non-selectable)
        let text_pos = rect.min + egui::vec2(8.0, 4.0);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_TOP,
            text,
            egui::FontId::default(),
            ui.style().visuals.text_color(),
        );

        response.clicked()
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
                        self.connected_host = Some(self.config.ssh.server.clone());
                        // Store server in local DB
                        if let Some(ref db) = self.db {
                            db.execute(
                                 "INSERT OR IGNORE INTO servers (host, user, key_path) VALUES (?1, ?2, ?3)",
                                 &[&self.config.ssh.server, &self.config.ssh.username, &self.config.ssh.ssh_key_path],
                             ).expect("Could not insert server");
                        }
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

        // Check for works results
        if let Ok(mut result) = self.works_result.try_lock() {
            if let Some(works_result) = result.take() {
                self.loading_works = false;
                match works_result {
                    Ok(works) => {
                        tracing::info!("Loaded {} works", works.len());
                        self.works = works;
                    }
                    Err(e) => {
                        tracing::error!("Failed to load works: {}", e);
                        self.connection_error = Some(format!("Failed to load works: {}", e));
                    }
                }
            }
        }

        // Check for work messages results
        if let Ok(mut result) = self.work_messages_result.try_lock() {
            if let Some(work_messages_result) = result.take() {
                self.loading_work_messages = false;
                match work_messages_result {
                    Ok(messages) => {
                        tracing::info!("Loaded {} work messages", messages.len());
                        self.work_messages = messages;
                    }
                    Err(e) => {
                        tracing::error!("Failed to load work messages: {}", e);
                        self.connection_error =
                            Some(format!("Failed to load work messages: {}", e));
                    }
                }
            }
        }

        // Check for AI session outputs results
        if let Ok(mut result) = self.ai_session_outputs_result.try_lock() {
            if let Some(ai_session_outputs_result) = result.take() {
                self.loading_ai_session_outputs = false;
                match ai_session_outputs_result {
                    Ok(outputs) => {
                        tracing::info!("Loaded {} AI session outputs", outputs.len());
                        self.ai_session_outputs = outputs;
                    }
                    Err(e) => {
                        tracing::error!("Failed to load AI session outputs: {}", e);
                        self.connection_error =
                            Some(format!("Failed to load AI session outputs: {}", e));
                    }
                }
            }
        }

        // Check for create work results
        if let Ok(mut result) = self.create_work_result.try_lock() {
            if let Some(create_work_result) = result.take() {
                self.creating_work = false;
                match create_work_result {
                    Ok(work) => {
                        tracing::info!("Created work: {} ({})", work.title, work.id);
                        // Add the new work to the list
                        self.works.push(work);
                        // Close the dialog and clear the form
                        self.show_new_work_dialog = false;
                        self.new_work_title.clear();
                        self.new_work_project_id = None;
                        self.new_work_model.clear();
                    }
                    Err(e) => {
                        tracing::error!("Failed to create work: {}", e);
                        self.connection_error = Some(format!("Failed to create work: {}", e));
                    }
                }
            }
        }

        // Auto-refresh projects and works after connection
        if should_refresh_projects {
            self.refresh_projects();
            self.refresh_works();
        }

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
                        let label = if let Some(host) = &self.connected_host {
                            format!("● Connected: {}", host)
                        } else {
                            "● Connected".to_string()
                        };
                        ui.colored_label(egui::Color32::GREEN, label);
                        ui.label(format!("Projects: {}", self.projects.len()));
                    }
                }

                if let Some(error) = &self.connection_error {
                    ui.colored_label(egui::Color32::RED, error);
                }
            });
        });

        // Left sidebar
        egui::SidePanel::left("sidebar")
            .exact_width(300.0)
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 2.0);
                ui.vertical(|ui| {
                    let sidebar_bg = ui.style().visuals.panel_fill;
                    let button_bg = ui.style().visuals.widgets.inactive.bg_fill;

                    // Branding
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("nocodo").size(20.0).strong());
                    ui.add_space(20.0);

                    // Top navigation
                    if self.sidebar_link(ui, "Projects", sidebar_bg, button_bg) {
                        self.current_page = Page::Projects;
                    }
                    if self.sidebar_link(ui, "Work", sidebar_bg, button_bg) {
                        self.current_page = Page::Work;
                    }
                    if self.sidebar_link(ui, "Mentions", sidebar_bg, button_bg) {
                        self.current_page = Page::Mentions;
                    }

                    // Empty space
                    ui.add_space(50.0);

                    // Bottom navigation
                    if self.sidebar_link(ui, "Servers", sidebar_bg, button_bg) {
                        self.current_page = Page::Servers;
                    }
                    if self.sidebar_link(ui, "Settings", sidebar_bg, button_bg) {
                        self.current_page = Page::Settings;
                    }
                });
            });

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_page {
                Page::Projects => {
                    ui.heading("Projects");

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
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.add_space(8.0);
                                    for project in &self.projects {
                                        // Card frame with padding and rounded corners
                                        egui::Frame::NONE
                                            .fill(ui.style().visuals.widgets.inactive.bg_fill)
                                            .corner_radius(8.0)
                                            .inner_margin(egui::Margin::same(12))
                                            .show(ui, |ui| {
                                                ui.vertical(|ui| {
                                                    // Project name - larger and bold
                                                    ui.label(egui::RichText::new(&project.name).size(16.0).strong());

                                                    ui.add_space(4.0);

                                                    // Project path - smaller, muted color
                                                    ui.label(egui::RichText::new(&project.path).size(12.0).color(ui.style().visuals.weak_text_color()));

                                                    // Description if present
                                                    if let Some(description) = &project.description {
                                                        ui.add_space(6.0);
                                                        ui.label(egui::RichText::new(description).size(11.0).color(ui.style().visuals.weak_text_color()));
                                                    }
                                                });
                                            });
                                        ui.add_space(8.0);
                                    }
                                });
                            }
                        }
                    }
                }
                Page::Work => {
                    ui.horizontal(|ui| {
                        ui.heading("Work");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("+ New Work").clicked() {
                                self.show_new_work_dialog = true;
                            }
                        });
                    });

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
                            if self.loading_works {
                                ui.vertical_centered(|ui| {
                                    ui.label("Loading work...");
                                    ui.add(egui::Spinner::new());
                                });
                            } else if self.works.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.label("No work found");
                                    if ui.button("Refresh").clicked() {
                                        self.refresh_works();
                                    }
                                });
                            } else {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.add_space(8.0);

                                    // Sort works by created_at (most recent first)
                                    let mut sorted_works = self.works.clone();
                                    sorted_works.sort_by(|a, b| b.created_at.cmp(&a.created_at));

                                    for work in &sorted_works {
                                        let work_id = work.id;

                                        // Full-width card frame with padding and rounded corners
                                        let response = egui::Frame::NONE
                                            .fill(ui.style().visuals.widgets.inactive.bg_fill)
                                            .corner_radius(8.0)
                                            .inner_margin(egui::Margin::same(12))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.vertical(|ui| {
                                                        // Work title - larger and bold
                                                        ui.label(egui::RichText::new(&work.title).size(16.0).strong());

                                                        ui.add_space(4.0);

                                                        // Metadata row
                                                        ui.horizontal(|ui| {
                                                            // Status badge
                                                            egui::Frame::NONE
                                                                .fill(ui.style().visuals.selection.bg_fill)
                                                                .corner_radius(4.0)
                                                                .inner_margin(egui::Margin::symmetric(8, 4))
                                                                .show(ui, |ui| {
                                                                    ui.label(egui::RichText::new(&work.status).size(11.0));
                                                                });

                                                            // Tool name if present
                                                            if let Some(tool_name) = &work.tool_name {
                                                                egui::Frame::NONE
                                                                    .fill(ui.style().visuals.selection.bg_fill)
                                                                    .corner_radius(4.0)
                                                                    .inner_margin(egui::Margin::symmetric(8, 4))
                                                                    .show(ui, |ui| {
                                                                        ui.label(egui::RichText::new(tool_name).size(11.0));
                                                                    });
                                                            }

                                                            // Model if present
                                                            if let Some(model) = &work.model {
                                                                egui::Frame::NONE
                                                                    .fill(ui.style().visuals.selection.bg_fill)
                                                                    .corner_radius(4.0)
                                                                    .inner_margin(egui::Margin::symmetric(8, 4))
                                                                    .show(ui, |ui| {
                                                                        ui.label(egui::RichText::new(model).size(11.0));
                                                                    });
                                                            }

                                                            // Project if linked
                                                            if let Some(project_id) = work.project_id {
                                                                if let Some(project) = self.projects.iter().find(|p| p.id == project_id) {
                                                                    egui::Frame::NONE
                                                                        .fill(ui.style().visuals.selection.bg_fill)
                                                                        .corner_radius(4.0)
                                                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                                                        .show(ui, |ui| {
                                                                            ui.label(egui::RichText::new(&project.name).size(11.0));
                                                                        });
                                                                }
                                                            }
                                                        });
                                                    });
                                                });
                                            });

                                        // Make the entire card clickable
                                        if response.response.interact(egui::Sense::click()).clicked() {
                                            self.current_page = Page::WorkDetail(work_id);
                                            self.refresh_work_messages(work_id);
                                        }

                                        // Change cursor to pointer on hover
                                        if response.response.hovered() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }

                                        ui.add_space(8.0);
                                    }
                                });
                            }
                        }
                    }
                }
                Page::WorkDetail(work_id) => {
                    // Find the work
                    let work = self.works.iter().find(|w| w.id == work_id).cloned();

                    if let Some(work) = work {
                        // Header with back button
                        ui.horizontal(|ui| {
                            if ui.button("← Back to Work List").clicked() {
                                self.current_page = Page::Work;
                            }
                        });

                        ui.add_space(8.0);

                        // Work title
                        ui.heading(&work.title);

                        ui.add_space(4.0);

                        // Work metadata
                        ui.horizontal(|ui| {
                            ui.label("Status:");
                            ui.label(&work.status);

                            if let Some(tool_name) = &work.tool_name {
                                ui.separator();
                                ui.label("Tool:");
                                ui.label(tool_name);
                            }

                            if let Some(model) = &work.model {
                                ui.separator();
                                ui.label("Model:");
                                ui.label(model);
                            }

                            if let Some(project_id) = work.project_id {
                                if let Some(project) = self.projects.iter().find(|p| p.id == project_id) {
                                    ui.separator();
                                    ui.label("Project:");
                                    ui.label(&project.name);
                                }
                            }
                        });

                        ui.separator();

                        // Message history
                        ui.heading("Message History");

                        match &self.connection_state {
                            ConnectionState::Disconnected => {
                                ui.vertical_centered(|ui| {
                                    ui.label("Not connected to server");
                                });
                            }
                            ConnectionState::Connecting => {
                                ui.vertical_centered(|ui| {
                                    ui.label("Connecting...");
                                    ui.add(egui::Spinner::new());
                                });
                            }
                            ConnectionState::Connected => {
                                if self.loading_work_messages || self.loading_ai_session_outputs {
                                    ui.vertical_centered(|ui| {
                                        ui.label("Loading messages...");
                                        ui.add(egui::Spinner::new());
                                    });
                                } else if self.work_messages.is_empty() && self.ai_session_outputs.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.label("No messages found");
                                        if ui.button("Refresh").clicked() {
                                            self.refresh_work_messages(work_id);
                                        }
                                    });
                                } else {
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        ui.add_space(8.0);

                                        // Combine and sort all messages by timestamp
                                        #[derive(Clone)]
                                        enum DisplayMessage {
                                            WorkMessage(manager_models::WorkMessage),
                                            AiOutput(manager_models::AiSessionOutput),
                                        }

                                        let mut all_messages: Vec<(i64, DisplayMessage)> = Vec::new();

                                        // Add work messages (user input)
                                        for msg in &self.work_messages {
                                            all_messages.push((msg.created_at, DisplayMessage::WorkMessage(msg.clone())));
                                        }

                                        // Add AI session outputs (AI responses)
                                        for output in &self.ai_session_outputs {
                                            all_messages.push((output.created_at, DisplayMessage::AiOutput(output.clone())));
                                        }

                                        // Sort by timestamp
                                        all_messages.sort_by_key(|(timestamp, _)| *timestamp);

                                        for (_timestamp, message) in &all_messages {
                                            match message {
                                                DisplayMessage::WorkMessage(msg) => {
                                                    // User message
                                                    let bg_color = ui.style().visuals.widgets.inactive.bg_fill;

                                                    egui::Frame::NONE
                                                        .fill(bg_color)
                                                        .corner_radius(8.0)
                                                        .inner_margin(egui::Margin::same(12))
                                                        .show(ui, |ui| {
                                                            ui.vertical(|ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.label(egui::RichText::new("User").size(12.0).strong());
                                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                        let datetime = chrono::DateTime::from_timestamp(msg.created_at, 0)
                                                                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                                                            .unwrap_or_else(|| "Unknown".to_string());
                                                                        ui.label(egui::RichText::new(datetime).size(10.0).color(ui.style().visuals.weak_text_color()));
                                                                    });
                                                                });
                                                                ui.add_space(4.0);
                                                                ui.label(&msg.content);
                                                            });
                                                        });
                                                }
                                                DisplayMessage::AiOutput(output) => {
                                                    // AI response message
                                                    let bg_color = ui.style().visuals.widgets.noninteractive.bg_fill;

                                                    egui::Frame::NONE
                                                        .fill(bg_color)
                                                        .corner_radius(8.0)
                                                        .inner_margin(egui::Margin::same(12))
                                                        .show(ui, |ui| {
                                                            ui.vertical(|ui| {
                                                                ui.horizontal(|ui| {
                                                                    // Determine label based on role and model
                                                                    let label = match (output.role.as_deref(), output.model.as_deref()) {
                                                                        (Some("tool"), _) => "nocodo".to_string(),
                                                                        (Some("assistant"), Some(model)) => format!("AI - {}", model),
                                                                        _ => "AI".to_string(),
                                                                    };
                                                                    ui.label(egui::RichText::new(label).size(12.0).strong());
                                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                        let datetime = chrono::DateTime::from_timestamp(output.created_at, 0)
                                                                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                                                            .unwrap_or_else(|| "Unknown".to_string());
                                                                        ui.label(egui::RichText::new(datetime).size(10.0).color(ui.style().visuals.weak_text_color()));
                                                                    });
                                                                });
                                                                ui.add_space(4.0);
                                                                ui.label(&output.content);
                                                            });
                                                        });
                                                }
                                            }
                                            ui.add_space(8.0);
                                        }
                                    });
                                }
                            }
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.label("Work not found");
                            if ui.button("Back to Work List").clicked() {
                                self.current_page = Page::Work;
                            }
                        });
                    }
                }
                Page::Mentions => {
                    ui.heading("Mentions");
                    ui.label("Dummy Mentions page");
                }
                Page::Servers => {
                    ui.heading("Servers");
                    if self.servers.is_empty() {
                        ui.label("No servers saved");
                    } else {
                        ui.label("Saved servers:");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for server in &self.servers {
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}@{}", server.user, server.host));
                                    if let Some(key_path) = &server.key_path {
                                        ui.separator();
                                        ui.label(format!("Key: {}", key_path));
                                    }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Connect").clicked() {
                                            self.config.ssh.server = server.host.clone();
                                            self.config.ssh.username = server.user.clone();
                                            self.config.ssh.ssh_key_path = server.key_path.clone().unwrap_or_default();
                                            self.show_connection_dialog = true;
                                        }
                                    });
                                });
                                ui.separator();
                            }
                        });
                    }
                }
                Page::Settings => {
                    ui.heading("Settings");
                    ui.label("Dummy Settings page");
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

        // New Work dialog
        if self.show_new_work_dialog {
            egui::Window::new("Create New Work")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(&mut self.new_work_title);

                    ui.label("Project (optional):");
                    egui::ComboBox::from_label("")
                        .selected_text(
                            self.new_work_project_id
                                .and_then(|id| self.projects.iter().find(|p| p.id == id))
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| "None".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.new_work_project_id, None, "None");
                            for project in &self.projects {
                                ui.selectable_value(
                                    &mut self.new_work_project_id,
                                    Some(project.id),
                                    &project.name,
                                );
                            }
                        });

                    ui.label("Model (optional):");
                    ui.text_edit_singleline(&mut self.new_work_model);

                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            if !self.new_work_title.trim().is_empty() {
                                self.create_work();
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_new_work_dialog = false;
                            self.new_work_title.clear();
                            self.new_work_project_id = None;
                            self.new_work_model.clear();
                        }
                    });

                    if self.creating_work {
                        ui.add(egui::Spinner::new());
                    }
                });
        }
    }
}
