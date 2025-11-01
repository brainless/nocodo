use crate::components::{AuthDialog, ConnectionDialog, Sidebar, StatusBar};
use crate::pages::{
    MentionsPage, Page, ProjectDetailPage, ProjectsPage, ServersPage, SettingsPage,
    UiReferencePage, UiTwoColumnMainContentPage, WorkPage,
};
use crate::services::{ApiService, BackgroundTasks};
use crate::state::ui_state::Page as UiPage;
use crate::state::{AppState, Server};
use eframe;
use rusqlite::Connection;
use std::sync::Arc;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DesktopApp {
    // Centralized state
    #[serde(skip)]
    state: AppState,

    // Pages
    #[serde(skip)]
    pages: std::collections::HashMap<UiPage, Box<dyn Page>>,

    // Components
    #[serde(skip)]
    sidebar: Sidebar,
    #[serde(skip)]
    status_bar: StatusBar,
    #[serde(skip)]
    connection_dialog: ConnectionDialog,
    #[serde(skip)]
    auth_dialog: AuthDialog,

    // Services
    #[serde(skip)]
    api_service: Arc<ApiService>,
    #[serde(skip)]
    background_tasks: BackgroundTasks,
}

impl Default for DesktopApp {
    fn default() -> Self {
        let mut app = Self {
            state: AppState::default(),
            pages: std::collections::HashMap::new(),
            sidebar: Sidebar,
            status_bar: StatusBar,
            connection_dialog: ConnectionDialog,
            auth_dialog: AuthDialog::default(),
            api_service: Arc::new(ApiService),
            background_tasks: BackgroundTasks::new(Arc::new(ApiService)),
        };

        // Initialize pages
        app.pages.insert(UiPage::Mentions, Box::new(MentionsPage));
        app.pages.insert(UiPage::Projects, Box::new(ProjectsPage));
        app.pages.insert(UiPage::Work, Box::new(WorkPage));
        app.pages.insert(UiPage::Servers, Box::new(ServersPage));
        app.pages.insert(UiPage::Settings, Box::new(SettingsPage));
        app.pages
            .insert(UiPage::UiReference, Box::new(UiReferencePage::default()));
        app.pages.insert(
            UiPage::UiTwoColumnMainContent,
            Box::new(UiTwoColumnMainContentPage::default()),
        );

        app
    }
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if authentication is required (401 detected)
        let auth_required_flag = self.state.connection_manager.get_auth_required_flag();
        if let Ok(mut auth_required) = auth_required_flag.lock() {
            if *auth_required {
                tracing::info!("Authentication required - showing auth dialog");
                self.state.ui_state.show_auth_dialog = true;
                *auth_required = false; // Reset flag
            }
        }

        // Handle pending project details refresh
        if let Some(project_id) = self.state.pending_project_details_refresh.take() {
            tracing::info!(
                "Processing pending_project_details_refresh for project_id={}",
                project_id
            );
            self.api_service
                .refresh_project_details(project_id, &mut self.state);
        }

        // Handle background tasks
        self.background_tasks
            .handle_background_updates(&mut self.state);

        // Status bar
        self.status_bar.ui(ctx, &self.state);

        // Sidebar
        if let Some(new_page) = self.sidebar.ui(ctx, &mut self.state) {
            self.navigate_to(new_page);
        }

        // Central panel - render current page
        egui::CentralPanel::default().show(ctx, |ui| {
            let current_page = self.state.ui_state.current_page.clone();

            // Handle ProjectDetail page specially since it needs the project_id
            // We don't store it in the HashMap since each instance needs a different project_id
            if let UiPage::ProjectDetail(project_id) = current_page {
                tracing::info!("Rendering ProjectDetail page for project_id={}", project_id);
                let mut detail_page = ProjectDetailPage::new(project_id);
                detail_page.ui(ctx, ui, &mut self.state);
            } else if let Some(page) = self.pages.get_mut(&current_page) {
                page.ui(ctx, ui, &mut self.state);
            }
        });

        // Connection dialog
        self.connection_dialog.ui(ctx, &mut self.state);

        // Auth dialog
        self.auth_dialog.ui(ctx, &mut self.state);
    }
}

impl DesktopApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure custom fonts
        Self::setup_fonts(&cc.egui_ctx);

        // Load previous app state (if any).
        let mut app: DesktopApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        // Load configuration
        app.state.config = crate::config::DesktopConfig::load().unwrap_or_default();

        // Initialize local database
        let config_dir = dirs::config_dir().expect("Could not find config dir");
        let nocodo_dir = config_dir.join("nocodo");
        std::fs::create_dir_all(&nocodo_dir).expect("Could not create nocodo config dir");
        let db_path = nocodo_dir.join("local.sqlite3");
        let db = Connection::open(&db_path).expect("Could not open DB");

        // Create tables
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
        )
        .expect("Could not create unique index");

        // Add port column if it doesn't exist (for backward compatibility)
        let _ = db.execute(
            "ALTER TABLE servers ADD COLUMN port INTEGER NOT NULL DEFAULT 22",
            [],
        );

        // Create favorites table
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

        // Load servers from database
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
            app.state.servers = server_iter.filter_map(|s| s.ok()).collect();
        }

        // Load favorites
        {
            let mut stmt = db
                .prepare("SELECT entity_type, entity_id FROM favorites")
                .expect("Could not prepare favorites statement");
            let favorites_iter = stmt
                .query_map([], |row| {
                    let entity_type: String = row.get(0)?;
                    let entity_id: i64 = row.get(1)?;
                    Ok((entity_type, entity_id))
                })
                .expect("Could not query favorites");

            for (entity_type, entity_id) in favorites_iter.flatten() {
                if entity_type == "project" {
                    app.state.favorite_projects.insert(entity_id);
                }
            }
        }

        app.state.db = Some(db);

        app
    }

    /// Configure custom fonts for the entire application
    ///
    /// Font families:
    /// - "ui_light" - Ubuntu Light for regular UI widgets (labels, navigation)
    /// - "ui_semibold" - Ubuntu SemiBold for emphasis (buttons, headings)
    /// - Proportional - Inter Regular for user content (project names, descriptions)
    /// - Monospace - Inter Medium for code blocks
    fn setup_fonts(ctx: &egui::Context) {
        // Load font files at compile time
        const UBUNTU_LIGHT: &[u8] = include_bytes!("../fonts/UbuntuSans-Light.ttf");
        const UBUNTU_SEMIBOLD: &[u8] = include_bytes!("../fonts/UbuntuSans-SemiBold.ttf");
        const INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
        const INTER_MEDIUM: &[u8] = include_bytes!("../fonts/Inter-Medium.ttf");

        let mut fonts = egui::FontDefinitions::default();

        // Install Ubuntu fonts for UI widgets
        fonts.font_data.insert(
            "ubuntu_light".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(UBUNTU_LIGHT)),
        );
        fonts.font_data.insert(
            "ubuntu_semibold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(UBUNTU_SEMIBOLD)),
        );

        // Install Inter fonts for user content
        fonts.font_data.insert(
            "inter_regular".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(INTER_REGULAR)),
        );
        fonts.font_data.insert(
            "inter_medium".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(INTER_MEDIUM)),
        );

        // Create custom font family for light UI widgets (labels, navigation, status)
        // Add emoji support by including default emoji font as fallback
        fonts.families.insert(
            egui::FontFamily::Name("ui_light".into()),
            vec![
                "ubuntu_light".to_owned(),
                "NotoEmoji-Regular".to_owned(), // Fallback for emojis
            ],
        );

        // Create custom font family for emphasized UI widgets (buttons, headings, CTAs)
        // Add emoji support by including default emoji font as fallback
        fonts.families.insert(
            egui::FontFamily::Name("ui_semibold".into()),
            vec![
                "ubuntu_semibold".to_owned(),
                "NotoEmoji-Regular".to_owned(), // Fallback for emojis
            ],
        );

        // Set Inter as the default font for user content (Proportional family)
        // This is used for project names, descriptions, file contents, etc.
        // Add emoji support by keeping the default emoji font as fallback
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "inter_regular".to_owned());
        // Ensure emoji font is present (should be there by default)
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("NotoEmoji-Regular".to_owned());

        // Set Inter Medium for code/monospace text
        // Add emoji support by keeping the default emoji font as fallback
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "inter_medium".to_owned());
        // Ensure emoji font is present (should be there by default)
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("NotoEmoji-Regular".to_owned());

        // Apply fonts to the context
        ctx.set_fonts(fonts);
    }

    fn navigate_to(&mut self, page: UiPage) {
        // Call on_navigate_from for current page
        if let Some(current_page) = self.pages.get_mut(&self.state.ui_state.current_page) {
            current_page.on_navigate_from();
        }

        // Update current page
        self.state.ui_state.current_page = page.clone();

        // Set refresh flags based on the page we're navigating to
        match &page {
            UiPage::Projects => {
                self.state.ui_state.pending_projects_refresh = true;
            }
            UiPage::Work => {
                self.state.ui_state.pending_works_refresh = true;
            }
            _ => {}
        }

        // Call on_navigate_to for new page
        if let Some(new_page) = self.pages.get_mut(&page) {
            new_page.on_navigate_to();
        }
    }
}
