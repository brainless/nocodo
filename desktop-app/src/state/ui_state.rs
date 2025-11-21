use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Page {
    Projects,
    Work,
    ProjectDetail(i64), // Project ID
    Mentions,
    Users,
    Teams,
    #[default]
    Servers,
    Settings,
    UiReference,
    UiTwoColumnMainContent,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub show_connection_dialog: bool,
    pub show_auth_dialog: bool,
    pub show_new_work_dialog: bool,
    pub new_work_title: String,
    pub new_work_project_id: Option<i64>,
    pub new_work_branch: Option<String>,
    pub new_work_model: Option<String>,
    pub connection_error: Option<String>,
    pub connected_host: Option<String>,
    pub current_page: Page,
    pub selected_work_id: Option<i64>,
    pub reset_work_details_scroll: bool,
    pub local_server_running: bool,
    pub checking_local_server: bool,
    pub projects_default_path: String,
    pub ui_reference_card_titles: Vec<String>,
    pub ui_reference_form_text: String,
    pub ui_reference_form_dropdown: Option<String>,
    pub ui_reference_readme_content: String,
    /// Set of expanded tool call IDs (for collapsible tool response widgets)
    #[serde(skip)]
    pub expanded_tool_calls: std::collections::HashSet<i64>,

    /// File management state for project detail page
    pub selected_file_path: Option<String>,
    pub expanded_folders: std::collections::HashSet<String>,
    /// Current directory path for file browser (None means root)
    pub current_directory_path: Option<String>,
    /// Message continuation input for work detail
    pub continue_message_input: String,
    /// Flags to trigger data refresh on page navigation
    #[serde(skip)]
    pub pending_projects_refresh: bool,
    #[serde(skip)]
    pub pending_works_refresh: bool,
    #[serde(skip)]
    pub pending_users_refresh: bool,
    #[serde(skip)]
    pub pending_teams_refresh: bool,
    /// Flag to trigger servers list refresh after successful SSH connection
    #[serde(skip)]
    pub servers_refresh_needed: Arc<std::sync::Mutex<bool>>,
    /// Flag to indicate if we're adding a new server (vs connecting to existing)
    #[serde(skip)]
    pub is_adding_new_server: bool,
    /// Flag to trigger navigation to Board after successful authentication
    #[serde(skip)]
    pub should_navigate_after_auth: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_connection_dialog: false,
            show_auth_dialog: false,
            show_new_work_dialog: false,
            new_work_title: String::new(),
            new_work_project_id: None,
            new_work_branch: None,
            new_work_model: None,
            connection_error: None,
            connected_host: None,
            current_page: Page::default(),
            selected_work_id: None,
            reset_work_details_scroll: false,
            local_server_running: false,
            checking_local_server: false,
            projects_default_path: String::new(),
            ui_reference_card_titles: Vec::new(),
            ui_reference_form_text: String::new(),
            ui_reference_form_dropdown: None,
            ui_reference_readme_content: String::new(),
            expanded_tool_calls: std::collections::HashSet::new(),

            selected_file_path: None,
            expanded_folders: std::collections::HashSet::new(),
            current_directory_path: None,
            continue_message_input: String::new(),
            pending_projects_refresh: false,
            pending_works_refresh: false,
            pending_users_refresh: false,
            pending_teams_refresh: false,
            servers_refresh_needed: Arc::new(std::sync::Mutex::new(false)),
            is_adding_new_server: false,
            should_navigate_after_auth: false,
        }
    }
}
