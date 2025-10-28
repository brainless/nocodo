use crate::state::AppState;
use std::sync::Arc;

pub struct BackgroundTasks {
    api_service: Arc<crate::services::ApiService>,
}

impl BackgroundTasks {
    pub fn new(api_service: Arc<crate::services::ApiService>) -> Self {
        Self { api_service }
    }
}

impl BackgroundTasks {
    pub fn handle_background_updates(&self, state: &mut AppState) {
        // Monitor connection state
        self.check_connection_state(state);

        // Handle async task results
        self.check_projects_result(state);
        self.check_works_result(state);
        self.check_work_messages_result(state);
        self.check_ai_session_outputs_result(state);
        self.check_ai_tool_calls_result(state);
        self.check_settings_result(state);
        self.check_project_details_result(state);
        self.check_supported_models_result(state);
        self.check_create_work_result(state);
        self.check_create_ai_session_result(state);
        self.check_update_api_keys_result(state);
        self.check_update_projects_path_result(state);
        self.check_scan_projects_result(state);
        self.check_local_server_result(state);
    }

    fn check_connection_state(&self, state: &mut AppState) {
        use crate::state::ConnectionState;

        // Check if connection result is available
        let result_opt = {
            let mut result = state.connection_result.lock().unwrap();
            result.take()
        };

        if let Some(res) = result_opt {
            match res {
                Ok(server) => {
                    tracing::info!("Connection successful to {}", server);
                    state.connection_state = ConnectionState::Connected;
                    state.ui_state.connected_host = Some(server);
                    state.ui_state.connection_error = None;
                    state.models_fetch_attempted = false; // Reset to allow fetching models

                    // Refresh data after connecting
                    let api_service = crate::services::ApiService::new();
                    api_service.refresh_settings(state);
                    api_service.refresh_projects(state);
                    api_service.refresh_works(state);
                    api_service.refresh_supported_models(state);
                }
                Err(error) => {
                    tracing::error!("Connection failed: {}", error);
                    state.connection_state = ConnectionState::Error(error.clone());
                    state.ui_state.connection_error = Some(error);
                    state.ui_state.connected_host = None;
                }
            }
        }
    }

    fn check_projects_result(&self, state: &mut AppState) {
        let mut result = state.projects_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.loading_projects = false;
            match res {
                Ok(projects) => {
                    state.projects = projects;
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to load projects: {}", e));
                }
            }
        }
    }

    fn check_works_result(&self, state: &mut AppState) {
        let mut result = state.works_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.loading_works = false;
            match res {
                Ok(works) => {
                    state.works = works;
                }
                Err(e) => {
                    state.ui_state.connection_error = Some(format!("Failed to load works: {}", e));
                }
            }
        }
    }

    fn check_work_messages_result(&self, state: &mut AppState) {
        let mut result = state.work_messages_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.loading_work_messages = false;
            match res {
                Ok(messages) => {
                    state.work_messages = messages;
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to load work messages: {}", e));
                }
            }
        }
    }

    fn check_ai_session_outputs_result(&self, state: &mut AppState) {
        let mut result = state.ai_session_outputs_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.loading_ai_session_outputs = false;
            match res {
                Ok(outputs) => {
                    state.ai_session_outputs = outputs;
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to load AI outputs: {}", e));
                }
            }
        }
    }

    fn check_ai_tool_calls_result(&self, state: &mut AppState) {
        let mut result = state.ai_tool_calls_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.loading_ai_tool_calls = false;
            match res {
                Ok(tool_calls) => {
                    state.ai_tool_calls = tool_calls;
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to load AI tool calls: {}", e));
                }
            }
        }
    }

    fn check_settings_result(&self, state: &mut AppState) {
        let mut result = state.settings_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.loading_settings = false;
            match res {
                Ok(settings) => {
                    state.settings = Some(settings.clone());
                    // Update projects default path from settings
                    if let Some(path) = &settings.projects_default_path {
                        state.ui_state.projects_default_path = path.clone();
                        state.projects_default_path_modified = false;
                    }
                    // Initialize API key input fields from loaded settings (unmask them)
                    // Note: We'll keep them empty initially for security, user needs to enter them
                    if state.xai_api_key_input.is_empty()
                        && state.openai_api_key_input.is_empty()
                        && state.anthropic_api_key_input.is_empty()
                    {
                        // Keep fields empty for security - don't populate from masked values
                        state.api_keys_modified = false;
                    }
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to load settings: {}", e));
                }
            }
        }
    }

    fn check_project_details_result(&self, state: &mut AppState) {
        let mut result = state.project_details_result.lock().unwrap();
        if let Some(res) = result.take() {
            tracing::info!("check_project_details_result: received result");
            state.loading_project_details = false;
            match res {
                Ok(details) => {
                    tracing::info!(
                        "Project details loaded successfully: project={}, {} components",
                        details.project.name,
                        details.components.len()
                    );
                    state.project_details = Some(details);
                }
                Err(e) => {
                    tracing::error!("Failed to load project details: {}", e);
                    state.ui_state.connection_error =
                        Some(format!("Failed to load project details: {}", e));
                }
            }
        }
    }

    fn check_supported_models_result(&self, state: &mut AppState) {
        let mut result = state.supported_models_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.loading_supported_models = false;
            match res {
                Ok(models) => {
                    state.supported_models = models;
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to load supported models: {}", e));
                }
            }
        }
    }

    fn check_create_work_result(&self, state: &mut AppState) {
        let result_opt = {
            let mut result = state.create_work_result.lock().unwrap();
            result.take()
        };

        if let Some(res) = result_opt {
            state.creating_work = false;
            match res {
                Ok(_work) => {
                    // Clear the form
                    state.ui_state.new_work_title.clear();
                    state.ui_state.new_work_project_id = None;
                    state.ui_state.new_work_model = None;
                    // Refresh works list to show the newly created work
                    self.api_service.refresh_works(state);
                }
                Err(e) => {
                    state.ui_state.connection_error = Some(format!("Failed to create work: {}", e));
                }
            }
        }
    }

    fn check_create_ai_session_result(&self, state: &mut AppState) {
        let mut result = state.create_ai_session_result.lock().unwrap();
        if let Some(res) = result.take() {
            match res {
                Ok(_session) => {
                    // AI session created successfully
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to create AI session: {}", e));
                }
            }
        }
    }

    fn check_update_api_keys_result(&self, state: &mut AppState) {
        let result_opt = {
            let mut result = state.update_api_keys_result.lock().unwrap();
            result.take()
        };

        if let Some(res) = result_opt {
            state.updating_api_keys = false;
            match res {
                Ok(_) => {
                    state.api_keys_modified = false;
                    // Refresh settings to get updated API key status
                    self.api_service.refresh_settings(state);
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to update API keys: {}", e));
                }
            }
        }
    }

    fn check_update_projects_path_result(&self, state: &mut AppState) {
        let result_opt = {
            let mut result = state.update_projects_path_result.lock().unwrap();
            result.take()
        };

        if let Some(res) = result_opt {
            state.updating_projects_path = false;
            match res {
                Ok(_) => {
                    state.projects_default_path_modified = false;
                    // Refresh settings to get updated path
                    self.api_service.refresh_settings(state);
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to update projects path: {}", e));
                }
            }
        }
    }

    fn check_scan_projects_result(&self, state: &mut AppState) {
        let result_opt = {
            let mut result = state.scan_projects_result.lock().unwrap();
            result.take()
        };

        if let Some(res) = result_opt {
            state.scanning_projects = false;
            match res {
                Ok(_) => {
                    // Refresh projects list after successful scan
                    self.api_service.refresh_projects(state);
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to scan projects: {}", e));
                }
            }
        }
    }

    fn check_local_server_result(&self, state: &mut AppState) {
        let mut result = state.local_server_check_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.ui_state.checking_local_server = false;
            state.ui_state.local_server_running = res;
        }
    }
}
