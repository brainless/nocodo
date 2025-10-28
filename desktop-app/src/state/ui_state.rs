use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Page {
    Projects,
    Work,
    ProjectDetail(i64), // Project ID
    Mentions,
    Servers,
    Settings,
    UiReference,
    UiTwoColumnMainContent,
}

impl Default for Page {
    fn default() -> Self {
        Page::Servers
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub show_connection_dialog: bool,
    pub show_new_work_dialog: bool,
    pub new_work_title: String,
    pub new_work_project_id: Option<i64>,
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
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_connection_dialog: false,
            show_new_work_dialog: false,
            new_work_title: String::new(),
            new_work_project_id: None,
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
        }
    }
}
