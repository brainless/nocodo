use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Page {
    Projects,
    Work,
    ProjectDetail(i64), // Project ID
    Mentions,
    #[default]
    Servers,
    Settings,
    UiReference,
    UiTwoColumnMainContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProjectDetailTab {
    #[default]
    Dashboard,
    Files,
    Components,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiState {
    pub show_connection_dialog: bool,
    pub show_auth_dialog: bool,
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
    /// Current selected tab in project detail page
    pub project_detail_tab: ProjectDetailTab,
    /// File management state for project detail page
    pub selected_file_path: Option<String>,
    pub expanded_folders: std::collections::HashSet<String>,
    /// Message continuation input for work detail
    pub continue_message_input: String,
}
