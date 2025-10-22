use rusqlite::Connection;
use serde_json::Value;
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
    new_work_model: Option<String>,
    connection_error: Option<String>,
    connected_host: Option<String>,

    // Data
    projects: Vec<manager_models::Project>,
    works: Vec<manager_models::Work>,
    work_messages: Vec<manager_models::WorkMessage>,
    ai_session_outputs: Vec<manager_models::AiSessionOutput>,
    project_details: Option<manager_models::ProjectDetailsResponse>,
    servers: Vec<Server>,
    settings: Option<manager_models::SettingsResponse>,
    supported_models: Vec<manager_models::SupportedModel>,
    current_page: Page,

    // Local server detection
    local_server_running: bool,
    #[serde(skip)]
    checking_local_server: bool,
    #[serde(skip)]
    local_server_check_result: Arc<std::sync::Mutex<Option<bool>>>,

    // Favorite state
    favorite_projects: std::collections::HashSet<i64>,

    // Projects default path state
    projects_default_path: String,
    projects_default_path_modified: bool,

    // Runtime state (not serialized)
    #[serde(skip)]
    tunnel: Option<crate::ssh::SshTunnel>,
    #[serde(skip)]
    api_client: Option<crate::api_client::ApiClient>,
    #[serde(skip)]
    pending_project_details_refresh: Option<i64>,
    #[serde(skip)]
    connection_result: Arc<std::sync::Mutex<Option<Result<crate::ssh::SshTunnel, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    settings_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::SettingsResponse, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    project_details_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::ProjectDetailsResponse, String>>>>,
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
    #[allow(clippy::type_complexity)]
    update_projects_path_result: Arc<std::sync::Mutex<Option<Result<Value, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    scan_projects_result: Arc<std::sync::Mutex<Option<Result<Value, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    supported_models_result: Arc<std::sync::Mutex<Option<Result<Vec<manager_models::SupportedModel>, String>>>>,
    #[serde(skip)]
    loading_projects: bool,
    loading_works: bool,
    loading_work_messages: bool,
    loading_ai_session_outputs: bool,
    loading_settings: bool,
    loading_project_details: bool,
    loading_supported_models: bool,
    models_fetch_attempted: bool,
    creating_work: bool,
    updating_projects_path: bool,
    scanning_projects: bool,
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
    WorkDetail(i64),    // Work ID
    ProjectDetail(i64), // Project ID
    Mentions,
    Servers,
    Settings,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Server {
    host: String,
    user: String,
    key_path: Option<String>,
    #[serde(default = "default_ssh_port")]
    port: u16,
}

fn default_ssh_port() -> u16 {
    22
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
            new_work_model: None,
            connection_error: None,
            connected_host: None,
            projects: Vec::new(),
            works: Vec::new(),
            work_messages: Vec::new(),
            ai_session_outputs: Vec::new(),
            project_details: None,
            servers: Vec::new(),
            settings: None,
            supported_models: Vec::new(),
            current_page: Page::Servers,
            local_server_running: false,
            checking_local_server: false,
            local_server_check_result: Arc::new(std::sync::Mutex::new(None)),
            favorite_projects: std::collections::HashSet::new(),
            projects_default_path: String::new(),
            projects_default_path_modified: false,
            tunnel: None,
            api_client: None,
            pending_project_details_refresh: None,
            connection_result: Arc::new(std::sync::Mutex::new(None)),
            projects_result: Arc::new(std::sync::Mutex::new(None)),
            works_result: Arc::new(std::sync::Mutex::new(None)),
            work_messages_result: Arc::new(std::sync::Mutex::new(None)),
            ai_session_outputs_result: Arc::new(std::sync::Mutex::new(None)),
            settings_result: Arc::new(std::sync::Mutex::new(None)),
            project_details_result: Arc::new(std::sync::Mutex::new(None)),
            supported_models_result: Arc::new(std::sync::Mutex::new(None)),
            loading_projects: false,
            loading_works: false,
            loading_work_messages: false,
            loading_ai_session_outputs: false,
            loading_settings: false,
            loading_project_details: false,
            loading_supported_models: false,
            models_fetch_attempted: false,
            creating_work: false,
            create_work_result: Arc::new(std::sync::Mutex::new(None)),
            updating_projects_path: false,
            scanning_projects: false,
            update_projects_path_result: Arc::new(std::sync::Mutex::new(None)),
            scan_projects_result: Arc::new(std::sync::Mutex::new(None)),
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
                key_path TEXT,
                port INTEGER NOT NULL DEFAULT 22
            )",
            [],
        )
        .expect("Could not create servers table");
        db.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_servers_unique ON servers (host, user, key_path, port)",
            [],
        ).expect("Could not create unique index");

        // Add port column if it doesn't exist (for backward compatibility)
        if let Err(_) = db.execute(
            "ALTER TABLE servers ADD COLUMN port INTEGER NOT NULL DEFAULT 22",
            [],
        ) {
            // Ignore error if column already exists
        }

        // Create favorites table for storing favorite entities
        db.execute(
            "CREATE TABLE IF NOT EXISTS favorites (
                id INTEGER PRIMARY KEY,
                entity_type TEXT NOT NULL,
                entity_id INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(entity_type, entity_id)
            )",
            [],
        )
        .expect("Could not create favorites table");
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_favorites_entity ON favorites (entity_type, entity_id)",
            [],
        )
        .expect("Could not create favorites index");

        // Load existing servers
        {
            let mut stmt = db
                .prepare("SELECT host, user, key_path, COALESCE(port, 22) FROM servers")
                .expect("Could not prepare statement");
            let server_iter = stmt
                .query_map([], |row| {
                    Ok(Server {
                        host: row.get(0)?,
                        user: row.get(1)?,
                        key_path: row.get(2)?,
                        port: row.get(3)?,
                    })
                })
                .expect("Could not query servers");
            app.servers = server_iter.filter_map(|s| s.ok()).collect();
        }

        app.db = Some(db);

        // Load favorite projects
        app.load_favorite_projects();

        // Always start disconnected - never restore connection state
        app.connection_state = ConnectionState::Disconnected;
        app.tunnel = None;
        app.api_client = None;
        app.pending_project_details_refresh = None;
        app.projects.clear();
        app.works.clear();
        app.work_messages.clear();
        app.ai_session_outputs.clear();
        app.project_details = None;
        app.connection_error = None;
        app.loading_projects = false;
        app.loading_works = false;
        app.loading_work_messages = false;
        app.loading_ai_session_outputs = false;
        
        // Always start on Servers page
        app.current_page = Page::Servers;

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
        let remote_port = self.config.ssh.remote_port;
        let port = self.config.ssh.port;

        // Spawn async task for SSH connection
        tokio::spawn(async move {
            let result = crate::ssh::SshTunnel::connect(
                &server,
                &username,
                key_path.as_deref(),
                port,
                remote_port,
            )
            .await;
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

    fn refresh_settings(&mut self) {
        if self.connection_state == ConnectionState::Connected {
            if let Some(ref api_client) = self.api_client {
                self.loading_settings = true;
                self.settings_result = Arc::new(std::sync::Mutex::new(None));

                let api_client = api_client.clone();
                let result_clone = Arc::clone(&self.settings_result);

                tokio::spawn(async move {
                    let result = api_client.get_settings().await;
                    let mut settings_result = result_clone.lock().unwrap();
                    *settings_result = Some(result.map_err(|e| e.to_string()));
                });
            }
        }
    }

    fn refresh_supported_models(&mut self) {
        if self.connection_state == ConnectionState::Connected {
            if let Some(ref api_client) = self.api_client {
                self.loading_supported_models = true;
                self.models_fetch_attempted = true;
                self.supported_models_result = Arc::new(std::sync::Mutex::new(None));

                let api_client = api_client.clone();
                let result_clone = Arc::clone(&self.supported_models_result);

                tokio::spawn(async move {
                    let result = api_client.get_supported_models().await;
                    let mut supported_models_result = result_clone.lock().unwrap();
                    *supported_models_result = Some(result.map_err(|e| e.to_string()));
                });
            }
        }
    }

    fn refresh_project_details(&mut self, project_id: i64) {
        if self.connection_state == ConnectionState::Connected {
            if let Some(ref api_client) = self.api_client {
                self.loading_project_details = true;
                self.project_details_result = Arc::new(std::sync::Mutex::new(None));

                let api_client = api_client.clone();
                let result_clone = Arc::clone(&self.project_details_result);

                tokio::spawn(async move {
                    let result = api_client.get_project_details(project_id).await;
                    let mut project_details_result = result_clone.lock().unwrap();
                    *project_details_result = Some(result.map_err(|e| e.to_string()));
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
                let model = self.new_work_model.clone();

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

    fn update_projects_default_path(&mut self) {
        if self.connection_state == ConnectionState::Connected {
            if let Some(ref api_client) = self.api_client {
                self.updating_projects_path = true;
                self.update_projects_path_result = Arc::new(std::sync::Mutex::new(None));

                let api_client = api_client.clone();
                let result_clone = Arc::clone(&self.update_projects_path_result);
                let path = self.projects_default_path.clone();

                tokio::spawn(async move {
                    let result = api_client.set_projects_default_path(path).await;
                    let mut update_result = result_clone.lock().unwrap();
                    *update_result = Some(result.map_err(|e| e.to_string()));
                });
            }
        }
    }

    fn scan_projects(&mut self) {
        if self.connection_state == ConnectionState::Connected {
            if let Some(ref api_client) = self.api_client {
                self.scanning_projects = true;
                self.scan_projects_result = Arc::new(std::sync::Mutex::new(None));

                let api_client = api_client.clone();
                let result_clone = Arc::clone(&self.scan_projects_result);

                tokio::spawn(async move {
                    let result = api_client.scan_projects().await;
                    let mut scan_result = result_clone.lock().unwrap();
                    *scan_result = Some(result.map_err(|e| e.to_string()));
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

    /// Load favorite projects from database
    fn load_favorite_projects(&mut self) {
        if let Some(ref db) = self.db {
            let mut stmt = db
                .prepare("SELECT entity_id FROM favorites WHERE entity_type = 'project'")
                .expect("Could not prepare statement for loading favorites");
            let favorite_iter = stmt
                .query_map([], |row| Ok(row.get::<_, i64>(0)?))
                .expect("Could not query favorites");
            self.favorite_projects = favorite_iter.filter_map(|f| f.ok()).collect();
        }
    }

    /// Toggle favorite status for a project
    fn toggle_project_favorite(&mut self, project_id: i64) {
        if let Some(ref db) = self.db {
            if self.favorite_projects.contains(&project_id) {
                // Remove from favorites
                db.execute(
                    "DELETE FROM favorites WHERE entity_type = 'project' AND entity_id = ?1",
                    [&project_id],
                )
                .expect("Could not delete favorite");
                self.favorite_projects.remove(&project_id);
            } else {
                // Add to favorites
                let now = chrono::Utc::now().timestamp();
                db.execute(
                    "INSERT OR IGNORE INTO favorites (entity_type, entity_id, created_at) VALUES ('project', ?1, ?2)",
                    [&project_id, &now],
                ).expect("Could not insert favorite");
                self.favorite_projects.insert(project_id);
            }
        }
    }

    /// Check if a project is favorited
    fn is_project_favorite(&self, project_id: i64) -> bool {
        self.favorite_projects.contains(&project_id)
    }

    /// Check if local nocodo manager is running
    fn check_local_server(&mut self) {
        self.checking_local_server = true;
        self.local_server_check_result = Arc::new(std::sync::Mutex::new(None));

        let result_clone = Arc::clone(&self.local_server_check_result);

        tokio::spawn(async move {
            // Try to connect to the local manager on default port 8081
            let result = reqwest::Client::new()
                .get("http://localhost:8081/api/health")
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await;

            let is_running = result.is_ok() && result.unwrap().status().is_success();

            let mut check_result = result_clone.lock().unwrap();
            *check_result = Some(is_running);
        });
    }
}

impl eframe::App for DesktopApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle pending project details refresh
        if let Some(project_id) = self.pending_project_details_refresh.take() {
            self.refresh_project_details(project_id);
        }

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
                        self.models_fetch_attempted = false; // Reset to allow fetching models on new connection
                        // Store server in local DB
                        if let Some(ref db) = self.db {
                            db.execute(
                                 "INSERT OR IGNORE INTO servers (host, user, key_path, port) VALUES (?1, ?2, ?3, ?4)",
                                 [&self.config.ssh.server, &self.config.ssh.username, &self.config.ssh.ssh_key_path, &self.config.ssh.port.to_string()],
                             ).expect("Could not insert server");
                        }
                        // Navigate to Projects page after successful connection
                        self.current_page = Page::Projects;
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

        // Check for settings results
        if let Ok(mut result) = self.settings_result.try_lock() {
            if let Some(settings_result) = result.take() {
                self.loading_settings = false;
                match settings_result {
                    Ok(settings) => {
                        tracing::info!("Loaded settings");
                        self.settings = Some(settings.clone());
                        // Update projects default path from settings
                        if let Some(path) = &settings.projects_default_path {
                            self.projects_default_path = path.clone();
                            self.projects_default_path_modified = false;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to load settings: {}", e);
                        self.connection_error = Some(format!("Failed to load settings: {}", e));
                    }
                }
            }
        }

        // Check for supported models results
        if let Ok(mut result) = self.supported_models_result.try_lock() {
            if let Some(models_result) = result.take() {
                self.loading_supported_models = false;
                match models_result {
                    Ok(models) => {
                        tracing::info!("Loaded {} supported models", models.len());
                        self.supported_models = models;
                    }
                    Err(e) => {
                        tracing::error!("Failed to load supported models: {}", e);
                        self.connection_error = Some(format!("Failed to load supported models: {}", e));
                    }
                }
            }
        }

        // Check for project details results
        if let Ok(mut result) = self.project_details_result.try_lock() {
            if let Some(project_details_result) = result.take() {
                self.loading_project_details = false;
                match project_details_result {
                    Ok(details) => {
                        tracing::info!("Loaded project details for project {}", details.project.id);
                        self.project_details = Some(details);
                    }
                    Err(e) => {
                        tracing::error!("Failed to load project details: {}", e);
                        self.connection_error =
                            Some(format!("Failed to load project details: {}", e));
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
                        // Clear the form
                        self.new_work_title.clear();
                        self.new_work_project_id = None;
                        self.new_work_model = None;
                    }
                    Err(e) => {
                        tracing::error!("Failed to create work: {}", e);
                        self.connection_error = Some(format!("Failed to create work: {}", e));
                    }
                }
            }
        }

        // Check for update projects path results
        let mut should_refresh_settings = false;
        if let Ok(mut result) = self.update_projects_path_result.try_lock() {
            if let Some(update_result) = result.take() {
                self.updating_projects_path = false;
                match update_result {
                    Ok(value) => {
                        tracing::info!("Updated projects default path");
                        // Update the local path with the expanded path from the API response
                        if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                            self.projects_default_path = path.to_string();
                        }
                        self.projects_default_path_modified = false;
                        should_refresh_settings = true;
                    }
                    Err(e) => {
                        tracing::error!("Failed to update projects path: {}", e);
                        self.connection_error =
                            Some(format!("Failed to update projects path: {}", e));
                    }
                }
            }
        }

        // Check for scan projects results
        let mut _should_refresh_projects = false;
        if let Ok(mut result) = self.scan_projects_result.try_lock() {
            if let Some(scan_result) = result.take() {
                self.scanning_projects = false;
                match scan_result {
                    Ok(_) => {
                        tracing::info!("Scanned projects successfully");
                        _should_refresh_projects = true;
                    }
                    Err(e) => {
                        tracing::error!("Failed to scan projects: {}", e);
                        self.connection_error = Some(format!("Failed to scan projects: {}", e));
                    }
                }
            }
        }

        // Check for local server check results
        if let Ok(mut result) = self.local_server_check_result.try_lock() {
            if let Some(check_result) = result.take() {
                self.checking_local_server = false;
                self.local_server_running = check_result;
                tracing::info!("Local server running: {}", self.local_server_running);
            }
        }

        // Refresh data after operations complete
        if should_refresh_settings {
            self.refresh_settings();
        }
        if should_refresh_projects {
            self.refresh_projects();
        }

        // Auto-refresh projects, works, settings, and models after connection
        if should_refresh_projects {
            self.refresh_projects();
            self.refresh_works();
            self.refresh_settings();
            self.refresh_supported_models();
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

                    // Favorite projects section
                    if !self.favorite_projects.is_empty() && self.connection_state == ConnectionState::Connected {
                        ui.add_space(4.0);
                        
                        // Show favorite projects
                        for project in &self.projects {
                            if self.favorite_projects.contains(&project.id) {
                                let available_width = ui.available_width();
                                let (rect, response) = 
                                    ui.allocate_exact_size(egui::vec2(available_width, 24.0), egui::Sense::click());
                                
                                // Change cursor to pointer on hover
                                if response.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                
                                // Determine background color based on hover state (same as sidebar_link)
                                let bg_color = if response.hovered() {
                                    button_bg
                                } else {
                                    sidebar_bg
                                };
                                
                                // Draw background with same border radius as sidebar_link (0.0)
                                ui.painter().rect_filled(rect, 0.0, bg_color);
                                
                                // Draw text with same styling as sidebar_link but with 12px left padding (8px + 4px extra)
                                let text_pos = rect.min + egui::vec2(12.0, 4.0); // Same y position (4.0) as sidebar_link
                                ui.painter().text(
                                    text_pos,
                                    egui::Align2::LEFT_TOP, // Same alignment as sidebar_link
                                    &project.name,
                                    egui::FontId::default(), // Same font as sidebar_link
                                    ui.style().visuals.text_color() // Same text color as sidebar_link
                                );
                                
                                // Handle click
                                if response.clicked() {
                                    self.current_page = Page::ProjectDetail(project.id);
                                    self.pending_project_details_refresh = Some(project.id);
                                }
                            }
                        }
                        ui.add_space(4.0);
                    }

                    if self.sidebar_link(ui, "Work", sidebar_bg, button_bg) {
                        self.current_page = Page::Work;
                        // Refresh works when navigating to Work page
                        if self.connection_state == ConnectionState::Connected
                            && self.works.is_empty()
                            && !self.loading_works
                        {
                            self.refresh_works();
                        }
                    }
                    if self.sidebar_link(ui, "Mentions", sidebar_bg, button_bg) {
                        self.current_page = Page::Mentions;
                    }

                    // Empty space
                    ui.add_space(50.0);

                    // Bottom navigation
                    if self.sidebar_link(ui, "Servers", sidebar_bg, button_bg) {
                        self.current_page = Page::Servers;
                        // Check local server when navigating to Servers page
                        if !self.checking_local_server {
                            self.check_local_server();
                        }
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

                                      let card_width = 300.0;
                                      let card_height = 100.0;
let card_spacing = 10.0;

                                      // Set spacing between items
                                      ui.spacing_mut().item_spacing = egui::Vec2::new(card_spacing, card_spacing);

                                      // Collect project IDs to avoid borrowing issues
                                      let project_ids: Vec<i64> = self.projects.iter().map(|p| p.id).collect();

                                      // Use horizontal_wrapped to create a responsive grid
                                      ui.horizontal_wrapped(|ui| {
                                          for (i, project) in self.projects.iter().enumerate() {
                                              let project_id = project_ids[i];
                                              // Use allocate_ui with fixed size to enable proper wrapping
                                              let response = ui.allocate_ui(egui::vec2(card_width, card_height), |ui| {
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
                                              });

                                              // Make the entire card clickable
                                              if response.response.interact(egui::Sense::click()).clicked() {
                                                  self.current_page = Page::ProjectDetail(project_id);
                                                  self.pending_project_details_refresh = Some(project_id);
                                              }

                                              // Change cursor to pointer on hover
                                              if response.response.hovered() {
                                                  ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                              }
                                          }
                                      });
                                  });
                             }
                        }
                    }
                }
                Page::Work => {
                    ui.heading("Work");

                    // Add the new work form directly on the page
                    if matches!(self.connection_state, ConnectionState::Connected) {
                        // Load models only once when form is opened
                        if !self.models_fetch_attempted && !self.loading_supported_models {
                            self.refresh_supported_models();
                        }
                        // Create form with same styling as work items
                        egui::Frame::NONE
                            .fill(ui.style().visuals.widgets.inactive.bg_fill)
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    // Title/Question field as textarea
                                    ui.label("What do you want to do?");
                                    ui.add_sized(
                                        egui::vec2(ui.available_width(), 60.0),
                                        egui::TextEdit::multiline(&mut self.new_work_title)
                                    );

                                    ui.add_space(8.0);

                                    // Project and Model fields side by side
                                    ui.horizontal(|ui| {
                                        // Project field
                                        ui.vertical(|ui| {
                                            ui.label("Project:");
                                            egui::ComboBox::from_id_salt("work_project_combo")
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
                                        });

                                        ui.add_space(16.0);

                                        // Model field
                                        ui.vertical(|ui| {
                                            ui.label("Model:");
                                            if self.loading_supported_models {
                                                ui.add(egui::Spinner::new());
                                            } else {
                                                egui::ComboBox::from_id_salt("work_model_combo")
                                                    .selected_text(
                                                        self.new_work_model
                                                            .as_ref()
                                                            .and_then(|model_id| self.supported_models.iter()
                                                                .find(|m| m.model_id == *model_id))
                                                            .map(|m| m.name.clone())
                                                            .unwrap_or_else(|| "None".to_string()),
                                                    )
                                                    .show_ui(ui, |ui| {
                                                        ui.selectable_value(&mut self.new_work_model, None, "None");
                                                        for model in &self.supported_models {
                                                            ui.selectable_value(
                                                                &mut self.new_work_model,
                                                                Some(model.model_id.clone()),
                                                                &model.name,
                                                            );
                                                        }
                                                    });
                                            }
                                        });
                                    });

                                    ui.add_space(8.0);

                                    // Show error if no models are configured
                                    if !self.loading_supported_models && self.supported_models.is_empty() {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("⚠").size(16.0).color(egui::Color32::from_rgb(255, 165, 0)));
                                            ui.label(
                                                egui::RichText::new("No models configured. Please set API keys in Settings page")
                                                    .color(egui::Color32::from_rgb(255, 165, 0))
                                            );
                                        });
                                        ui.add_space(8.0);
                                    }

                                    // Create button
                                    ui.horizontal(|ui| {
                                        if ui.button("Create").clicked() && !self.new_work_title.trim().is_empty() {
                                            self.create_work();
                                        }

                                        if self.creating_work {
                                            ui.add(egui::Spinner::new());
                                        }
                                    });
                                });
                            });

                        ui.add_space(16.0);
                    }

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
Page::ProjectDetail(project_id) => {
                      // Header with back button and star button
                      ui.horizontal(|ui| {
                          if ui.button("← Back to Projects").clicked() {
                              self.current_page = Page::Projects;
}

                           ui.add_space(10.0);

                           // Star button
                          let is_favorite = self.is_project_favorite(project_id);
                          let star_text = if is_favorite { "⭐ Star" } else { "☆ Star" };
let star_color = if is_favorite {
                               egui::Color32::YELLOW
                           } else {
                               ui.style().visuals.text_color()
                           };

                           if ui.button(egui::RichText::new(star_text).color(star_color)).clicked() {
                              self.toggle_project_favorite(project_id);
                          }
                      });

                     ui.add_space(8.0);

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
                             if self.loading_project_details {
                                 ui.vertical_centered(|ui| {
                                     ui.label("Loading project details...");
                                     ui.add(egui::Spinner::new());
                                 });
                             } else if let Some(details) = &self.project_details {
                                 // Project title
                                 ui.heading(&details.project.name);

                                 ui.add_space(4.0);

                                 // Project metadata
                                 ui.horizontal(|ui| {
                                     ui.label("Path:");
                                     ui.label(&details.project.path);

                                     if let Some(description) = &details.project.description {
                                         ui.separator();
                                         ui.label("Description:");
                                         ui.label(description);
                                     }
                                 });

                                 ui.separator();

                                 // Project components
                                 ui.heading("Project Components");

                                 if details.components.is_empty() {
                                     ui.label("No components found");
                                 } else {
                                     egui::ScrollArea::vertical().show(ui, |ui| {
                                         for component in &details.components {
                                             ui.horizontal(|ui| {
                                                 ui.label(&component.name);
                                                 ui.separator();
                                                 ui.label(&component.path);
                                                 ui.separator();
                                                 ui.label(&component.language);
                                                 if let Some(framework) = &component.framework {
                                                     ui.separator();
                                                     ui.label(framework);
                                                 }
                                             });
                                             ui.separator();
                                         }
                                     });
                                 }

                                 ui.add_space(8.0);

                                 if ui.button("Refresh").clicked() {
                                     self.refresh_project_details(project_id);
                                 }
                             } else {
                                 ui.vertical_centered(|ui| {
                                     ui.label("Project not found");
                                     if ui.button("Back to Projects").clicked() {
                                         self.current_page = Page::Projects;
                                     }
                                 });
                             }
                         }
                     }
                 }
                Page::Mentions => {
                    ui.heading("Mentions");
                    ui.label("Dummy Mentions page");
                }
Page::Servers => {
                    ui.heading("Servers");

                    // Local server section
ui.heading("Local server:");

                     if ui.button("Refresh Local Server Status").clicked() {
                        self.check_local_server();
                    }

                     if self.checking_local_server {
                        ui.horizontal(|ui| {
                            ui.label("Checking local server...");
                            ui.add(egui::Spinner::new());
                        });
                    } else if self.local_server_running {
                        // Show grid with localhost entry
ui.label("Local nocodo manager is running:");

                         let card_width = 300.0;
                         let card_height = 60.0;
                         let _card_spacing = 10.0;

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
                                                ui.label(egui::RichText::new("No key required").size(12.0).color(ui.style().visuals.weak_text_color()));
                                            });
                                        });
                                    });
});

                             // Make the card clickable
                            if response.response.interact(egui::Sense::click()).clicked() {
                                // Connect directly to local server without SSH
                                self.api_client = Some(crate::api_client::ApiClient::new("http://localhost:8081".to_string()));
                                self.connection_state = ConnectionState::Connected;
                                self.connected_host = Some("localhost".to_string());
                                self.tunnel = None; // No SSH tunnel for local connection
                                self.models_fetch_attempted = false; // Reset to allow fetching models on new connection
                                // Refresh data after connecting
                                self.refresh_projects();
                                self.refresh_works();
self.refresh_settings();
                             }

                             // Change cursor to pointer on hover
                            if response.response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        });
                    } else {
                        ui.label("You can run nocodo manager locally on this computer and connect to it");
                        ui.label("Start the manager with: nocodo-manager --config ~/.config/nocodo/manager.toml");
                    }

                    ui.add_space(20.0);

                    // Saved servers section
ui.heading("Saved servers:");

                     if self.servers.is_empty() {
                        ui.label("No servers saved");
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui_extras::TableBuilder::new(ui)
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
                                    for server in &self.servers {
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
                                                    ui.label(egui::RichText::new("Default").color(ui.style().visuals.weak_text_color()));
                                                }
                                            });
                                            row.col(|ui| {
                                                if ui.button("Connect").clicked() {
                                                    self.config.ssh.server = server.host.clone();
                                                    self.config.ssh.username = server.user.clone();
                                                    self.config.ssh.port = server.port;
                                                    self.config.ssh.ssh_key_path = server.key_path.clone().unwrap_or_default();
                                                    self.show_connection_dialog = true;
                                                }
                                            });
                                        });
                                    }
                                });
                        });
                    }
                }
                 Page::Settings => {
                     ui.heading("Settings");

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
                             if self.loading_settings {
                                 ui.vertical_centered(|ui| {
                                     ui.label("Loading settings...");
                                     ui.add(egui::Spinner::new());
                                 });
                             } else if let Some(settings) = &self.settings {
                                 ui.heading("API Keys");

                                 egui::ScrollArea::vertical().show(ui, |ui| {
                                     for api_key in &settings.api_keys {
                                         ui.vertical(|ui| {
                                             ui.label(&api_key.name);
                                             ui.add(
                                                 egui::TextEdit::singleline(&mut api_key.key.as_ref().unwrap_or(&String::new()).clone())
                                                     .desired_width(300.0)
                                                     .interactive(false)
                                             );
                                             ui.horizontal(|ui| {
                                                 ui.label(if api_key.is_configured { "✓ Configured" } else { "✗ Not configured" });
                                             });
                                         });
                                         ui.separator();
                                     }
                                 });

                                  ui.add_space(20.0);
                                  ui.heading("Projects Default Path");

                                  ui.horizontal(|ui| {
                                      ui.label("Path:");
                                      let response = ui.text_edit_singleline(&mut self.projects_default_path);
                                      if response.changed() {
                                          self.projects_default_path_modified = true;
                                      }
                                  });

                                  ui.horizontal(|ui| {
                                      let update_button = ui.add_enabled(
                                          self.projects_default_path_modified && !self.updating_projects_path,
                                          egui::Button::new("Update path")
                                      );

                                      if update_button.clicked() {
                                          self.update_projects_default_path();
                                      }

                                      if self.updating_projects_path {
                                          ui.add(egui::Spinner::new());
                                      }

                                      ui.add_space(10.0);

                                      let scan_button = ui.add_enabled(
                                          !self.scanning_projects,
                                          egui::Button::new("Scan and load projects")
                                      );

                                      if scan_button.clicked() {
                                          self.scan_projects();
                                      }

                                      if self.scanning_projects {
                                          ui.add(egui::Spinner::new());
                                      }
                                  });

                                  if ui.button("Refresh Settings").clicked() {
                                      self.refresh_settings();
                                  }
                               } else {
                                   ui.vertical_centered(|ui| {
                                       ui.label("No settings loaded");
                                       if ui.button("Refresh").clicked() {
                                           self.refresh_settings();
                                       }
                                   });
                               }
                          }
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

                    ui.label("Port:");
                    ui.add(egui::DragValue::new(&mut self.config.ssh.port).range(1..=65535));

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
