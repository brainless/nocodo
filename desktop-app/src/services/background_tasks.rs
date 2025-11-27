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
        self.check_worktree_branches_result(state);
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
        self.check_file_list_result(state);
        self.check_file_content_result(state);
        self.check_send_message_result(state);
        self.check_users_result(state);
        self.check_teams_result(state);
        self.check_update_user_result(state);
        self.check_update_team_result(state);
        self.check_login_result(state);
        self.check_current_user_teams_result(state);
        self.check_add_ssh_key_result(state);
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
                Ok((server, username, port)) => {
                    tracing::info!("Connection successful to {}@{}:{}", username, server, port);
                    state.connection_state = ConnectionState::Connected;
                    state.ui_state.connected_host = Some(server.clone());
                    state.ui_state.connection_error = None;
                    state.current_server_info = Some((server.clone(), username.clone(), port));
                    tracing::info!("Set current_server_info for SSH connection: {:?}", state.current_server_info);
                    tracing::info!("Favorites in memory: {:?}", state.favorite_projects);
                    state.models_fetch_attempted = false; // Reset to allow fetching models

                    // Automatically show auth dialog after successful SSH connection
                    // (unless already authenticated)
                    let should_show_auth = state.auth_state.jwt_token.is_none();
                    if should_show_auth {
                        tracing::info!("SSH connection successful to {}, showing auth dialog", server);
                        // Directly set the flag on the ui_state instead of using connection_manager
                        state.ui_state.show_auth_dialog = true;
                    }

                    // Don't navigate yet - wait for authentication (projects load) to complete
                    // Navigation will happen in check_projects_result after successful auth

                    // Note: Don't refresh data here - all API calls require auth
                    // They will be called after successful login in check_login_result
                }
                Err(error) => {
                    tracing::error!("Connection failed: {}", error);
                    state.connection_state = ConnectionState::Error(error.clone());
                    state.ui_state.connection_error = Some(error.clone());
                    state.ui_state.connected_host = None;
                }
            }
        }
    }

    fn check_projects_result(&self, state: &mut AppState) {
        use crate::state::ui_state::Page as UiPage;

        let mut result = state.projects_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.loading_projects = false;
            match res {
                Ok(projects) => {
                    let was_empty = state.projects.is_empty();
                    state.projects = projects;
                    // Clear any previous connection errors since we successfully loaded data
                    state.ui_state.connection_error = None;

                    // If this is the first successful projects load (indicating successful auth)
                    // and we're on the Servers page, and the user explicitly triggered auth,
                    // then navigate to Board page
                    if was_empty
                        && !state.projects.is_empty()
                        && state.ui_state.current_page == UiPage::Servers
                        && state.ui_state.should_navigate_after_auth
                    {
                        tracing::info!("Authentication successful, navigating to Board page");
                        state.ui_state.current_page = UiPage::Work;
                        state.ui_state.pending_works_refresh = true;
                        state.ui_state.should_navigate_after_auth = false; // Reset flag
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to load projects: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
                }
            }
        }
    }

    fn check_worktree_branches_result(&self, state: &mut AppState) {
        let mut result = state.worktree_branches_result.lock().unwrap();
        if let Some(res) = result.take() {
            state.loading_worktree_branches = false;
            state.worktree_branches_fetch_attempted = true;
            match res {
                Ok(branches) => {
                    state.worktree_branches = branches;
                }
                Err(e) => {
                    state.ui_state.connection_error =
                        Some(format!("Failed to load worktree branches: {}", e));
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
                    let error_msg = format!("Failed to load works: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to load work messages: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to load AI outputs: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to load AI tool calls: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to load settings: {}", e);
                    // Only show settings error in status bar if we have no projects loaded
                    // (meaning we're not properly authenticated yet)
                    if state.projects.is_empty() {
                        tracing::error!("{}", error_msg);
                        state.ui_state.connection_error = Some(error_msg);
                    } else {
                        // Log the error but don't show it in status bar since we're authenticated
                        tracing::warn!("Failed to load settings (non-critical): {}", e);
                    }
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
                    let error_msg = format!("Failed to load project details: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to load supported models: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to create work: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to create AI session: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to update API keys: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to update projects path: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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
                    let error_msg = format!("Failed to scan projects: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
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

    fn check_file_list_result(&self, state: &mut AppState) {
        let result = state.file_list_result.lock().unwrap();
        if result.is_some() {
            state.loading_file_list = false;
        }
    }

    fn check_file_content_result(&self, state: &mut AppState) {
        let result = state.file_content_result.lock().unwrap();
        if result.is_some() {
            state.loading_file_content = false;
        }
    }

    fn check_send_message_result(&self, state: &mut AppState) {
        let result_opt = {
            let mut result = state.send_message_result.lock().unwrap();
            result.take()
        };

        if let Some(res) = result_opt {
            state.sending_message = false;
            match res {
                Ok(_message) => {
                    // Clear the input
                    state.ui_state.continue_message_input.clear();
                    // Refresh messages to show the new message and AI response
                    if let Some(work_id) = state.ui_state.selected_work_id {
                        self.api_service.refresh_work_messages(work_id, state);
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to send message: {}", e);
                    tracing::error!("{}", error_msg);
                    state.ui_state.connection_error = Some(error_msg);
                }
            }
        }
    }

    fn check_users_result(&self, state: &mut AppState) {
        // Check users result
        let mut users_updated = false;
        {
            let mut result = state.users_result.lock().unwrap();
            if let Some(result) = result.take() {
                state.loading_users = false;
                match result {
                    Ok(users) => {
                        state.users = users;
                        users_updated = true;
                        // Clear any previous connection errors since we successfully loaded data
                        state.ui_state.connection_error = None;
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to load users: {}", e);
                        tracing::error!("{}", error_msg);
                        state.ui_state.connection_error = Some(error_msg);
                    }
                }
            }
        }
        if users_updated {
            self.apply_user_search_filter(state);
        }
    }

    fn check_teams_result(&self, state: &mut AppState) {
        // Check teams result
        let mut teams_updated = false;
        {
            let mut result = state.teams_result.lock().unwrap();
            if let Some(result) = result.take() {
                state.loading_teams = false;
                match result {
                    Ok(team_list_items) => {
                        state.team_list_items = team_list_items.clone();
                        teams_updated = true;
                        // Clear any previous connection errors since we successfully loaded data
                        state.ui_state.connection_error = None;
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to load teams: {}", e);
                        tracing::error!("{}", error_msg);
                        state.ui_state.connection_error = Some(error_msg);
                    }
                }
            }
        }
        if teams_updated {
            self.apply_team_search_filter(state);
        }
    }

    fn check_update_user_result(&self, state: &mut AppState) {
        // Check update user result
        let mut refresh_users = false;
        {
            let mut result = state.update_user_result.lock().unwrap();
            if let Some(result) = result.take() {
                state.updating_user = false;
                match result {
                    Ok(_) => {
                        state.show_user_modal = false;
                        refresh_users = true;
                        // Clear any previous connection errors since we successfully loaded data
                        state.ui_state.connection_error = None;
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to update user: {}", e);
                        tracing::error!("{}", error_msg);
                        state.ui_state.connection_error = Some(error_msg);
                    }
                }
            }
        }
        if refresh_users {
            self.api_service.refresh_users(state); // Refresh list
        }
    }

    fn apply_user_search_filter(&self, state: &mut AppState) {
        let query = state.user_search_query.to_lowercase();
        if query.is_empty() {
            state.filtered_users = state.users.clone();
        } else {
            state.filtered_users = state
                .users
                .iter()
                .filter(|u| {
                    u.name.to_lowercase().contains(&query)
                        || u.email.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
    }

    fn check_update_team_result(&self, state: &mut AppState) {
        // Check update team result
        let mut refresh_teams = false;
        {
            let mut result = state.update_team_result.lock().unwrap();
            if let Some(result) = result.take() {
                state.updating_team = false;
                match result {
                    Ok(_) => {
                        state.show_team_modal = false;
                        refresh_teams = true;
                        // Clear any previous connection errors since we successfully loaded data
                        state.ui_state.connection_error = None;
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to update team: {}", e);
                        tracing::error!("{}", error_msg);
                        state.ui_state.connection_error = Some(error_msg);
                    }
                }
            }
        }
        if refresh_teams {
            self.api_service.refresh_team_list(state); // Refresh list
        }
    }

    fn apply_team_search_filter(&self, state: &mut AppState) {
        let query = state.team_search_query.to_lowercase();
        if query.is_empty() {
            state.filtered_teams = state.team_list_items.clone();
        } else {
            state.filtered_teams = state
                .team_list_items
                .iter()
                .filter(|t| {
                    t.name.to_lowercase().contains(&query)
                        || t.description
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&query))
                })
                .cloned()
                .collect();
        }
    }

    fn check_login_result(&self, state: &mut AppState) {
        let result_opt = {
            let mut result = state.login_result.lock().unwrap();
            result.take()
        };

        if let Some(res) = result_opt {
            match res {
                Ok(login_response) => {
                    // Update auth_state with login info
                    state.auth_state.jwt_token = Some(login_response.token);
                    state.auth_state.user_id = Some(login_response.user.id);
                    state.auth_state.username = Some(login_response.user.username);
                    tracing::info!("Auth state updated after successful login");

                    // Load favorites for the current server (must be done after current_server_info is set)
                    state.load_favorites_for_current_server();

                    // Now that we're authenticated, fetch all data that requires auth
                    self.api_service.refresh_settings(state);
                    self.api_service.refresh_projects(state);
                    self.api_service.refresh_works(state);
                    self.api_service.refresh_supported_models(state);
                    self.api_service.refresh_current_user_teams(state);
                }
                Err(e) => {
                    tracing::error!("Login failed: {}", e);
                    state.ui_state.connection_error = Some(format!("Login failed: {}", e));
                }
            }
        }
    }

    fn check_current_user_teams_result(&self, state: &mut AppState) {
        let result_opt = {
            let mut result = state.current_user_teams_result.lock().unwrap();
            result.take()
        };

        if let Some(res) = result_opt {
            state.loading_current_user_teams = false;
            match res {
                Ok(teams) => {
                    state.current_user_teams = teams;
                }
                Err(e) => {
                    tracing::error!("Failed to load current user teams: {}", e);
                    // Don't show error to user, just silently fail - the section won't show
                }
            }
        }
    }

    fn check_add_ssh_key_result(&self, state: &mut AppState) {
        let result_opt = {
            let mut result = state.add_ssh_key_result.lock().unwrap();
            result.take()
        };

        if let Some(res) = result_opt {
            state.adding_ssh_key = false;
            match res {
                Ok(message) => {
                    state.ssh_key_message = Some(message);
                    state.ssh_public_key_input.clear();
                }
                Err(e) => {
                    state.ssh_key_message = Some(format!("Error: {}", e));
                }
            }
        }
    }
}
