use crate::components::{ConnectionDialog, Sidebar, StatusBar};
use crate::pages::{
    MentionsPage, Page, ProjectDetailPage, ProjectsPage, ServersPage, SettingsPage,
    UiReferencePage, UiTwoColumnMainContentPage, WorkPage,
};
use crate::services::{ApiService, BackgroundTasks};
use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use eframe;
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
            sidebar: Sidebar::default(),
            status_bar: StatusBar::default(),
            connection_dialog: ConnectionDialog::default(),
            api_service: Arc::new(ApiService::default()),
            background_tasks: BackgroundTasks::new(Arc::new(ApiService::default())),
        };

        // Initialize pages
        app.pages
            .insert(UiPage::Mentions, Box::new(MentionsPage::default()));
        app.pages
            .insert(UiPage::Projects, Box::new(ProjectsPage::default()));
        app.pages
            .insert(UiPage::Work, Box::new(WorkPage::default()));
        app.pages
            .insert(UiPage::Servers, Box::new(ServersPage::default()));
        app.pages
            .insert(UiPage::Settings, Box::new(SettingsPage::default()));
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
            if let UiPage::ProjectDetail(project_id) = current_page {
                if let Some(page) = self.pages.get_mut(&UiPage::ProjectDetail(0)) {
                    // Create a temporary ProjectDetailPage with the correct ID
                    let mut detail_page = ProjectDetailPage::new(project_id);
                    detail_page.ui(ctx, ui, &mut self.state);
                }
            } else if let Some(page) = self.pages.get_mut(&current_page) {
                page.ui(ctx, ui, &mut self.state);
            }
        });

        // Connection dialog
        self.connection_dialog.ui(ctx, &mut self.state);
    }
}

impl DesktopApp {
    fn navigate_to(&mut self, page: UiPage) {
        // Call on_navigate_from for current page
        if let Some(current_page) = self.pages.get_mut(&self.state.ui_state.current_page) {
            current_page.on_navigate_from();
        }

        // Update current page
        self.state.ui_state.current_page = page.clone();

        // Call on_navigate_to for new page
        if let Some(new_page) = self.pages.get_mut(&page) {
            new_page.on_navigate_to();
        }
    }
}
