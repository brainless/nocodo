use crate::state::AppState;
use std::sync::Arc;

pub struct ApiService;

impl ApiService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiService {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiService {
    // Connection methods
    pub async fn connect_to_server(&self, state: &mut AppState) -> Result<(), String> {
        // This will be implemented with actual API calls
        state.connection_state = crate::state::ConnectionState::Connected;
        state.ui_state.connected_host = Some(state.config.ssh.server.clone());
        Ok(())
    }

    pub async fn connect_to_local_server(&self, state: &mut AppState) -> Result<(), String> {
        // This will be implemented with actual API calls
        let connection_manager = Arc::clone(&state.connection_manager);
        match connection_manager.connect_local(8081).await {
            Ok(_) => {
                state.connection_state = crate::state::ConnectionState::Connected;
                state.ui_state.connected_host = Some("localhost".to_string());
                state.models_fetch_attempted = false;
                Ok(())
            }
            Err(e) => Err(format!("Failed to connect to local manager: {}", e)),
        }
    }

    // Project methods
    pub fn refresh_projects(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_projects = true;
            state.projects_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.projects_result);

            tokio::spawn(async move {
                if let Some(api_client) = connection_manager.get_api_client().await {
                    let result = api_client.list_projects().await;
                    let mut projects_result = result_clone.lock().unwrap();
                    *projects_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut projects_result = result_clone.lock().unwrap();
                    *projects_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_works(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_works = true;
            state.works_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.works_result);

            tokio::spawn(async move {
                if let Some(api_client) = connection_manager.get_api_client().await {
                    let result = api_client.list_works().await;
                    let mut works_result = result_clone.lock().unwrap();
                    *works_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut works_result = result_clone.lock().unwrap();
                    *works_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_settings(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_settings = true;
            state.settings_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.settings_result);

            tokio::spawn(async move {
                if let Some(api_client) = connection_manager.get_api_client().await {
                    let result = api_client.get_settings().await;
                    let mut settings_result = result_clone.lock().unwrap();
                    *settings_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut settings_result = result_clone.lock().unwrap();
                    *settings_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_supported_models(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_supported_models = true;
            state.models_fetch_attempted = true;
            state.supported_models_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.supported_models_result);

            tokio::spawn(async move {
                if let Some(api_client) = connection_manager.get_api_client().await {
                    let result = api_client.get_supported_models().await;
                    let mut supported_models_result = result_clone.lock().unwrap();
                    *supported_models_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut supported_models_result = result_clone.lock().unwrap();
                    *supported_models_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn create_work(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.creating_work = true;
            state.create_work_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.create_work_result);

            let title = state.ui_state.new_work_title.clone();
            let project_id = state.ui_state.new_work_project_id;
            let model = state.ui_state.new_work_model.clone();

            tokio::spawn(async move {
                if let Some(api_client) = connection_manager.get_api_client().await {
                    let request = manager_models::CreateWorkRequest {
                        title,
                        project_id,
                        model,
                        auto_start: true, // Automatically start LLM agent session
                        tool_name: Some("llm-agent".to_string()),
                    };
                    let result = api_client.create_work(request).await;
                    let mut create_work_result = result_clone.lock().unwrap();
                    *create_work_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut create_work_result = result_clone.lock().unwrap();
                    *create_work_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_work_messages(&self, work_id: i64, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            // Fetch both work messages and AI session outputs
            state.loading_work_messages = true;
            state.loading_ai_session_outputs = true;
            state.work_messages_result = Arc::new(std::sync::Mutex::new(None));
            state.ai_session_outputs_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let connection_manager2 = Arc::clone(&state.connection_manager);
            let messages_result_clone = Arc::clone(&state.work_messages_result);
            let outputs_result_clone = Arc::clone(&state.ai_session_outputs_result);

            // Fetch work messages (user input)
            tokio::spawn(async move {
                if let Some(api_client) = connection_manager.get_api_client().await {
                    let result = api_client.get_work_messages(work_id).await;
                    let mut work_messages_result = messages_result_clone.lock().unwrap();
                    *work_messages_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut work_messages_result = messages_result_clone.lock().unwrap();
                    *work_messages_result = Some(Err("Not connected".to_string()));
                }
            });

            // Fetch AI session outputs (AI responses and tool results)
            tokio::spawn(async move {
                if let Some(api_client) = connection_manager2.get_api_client().await {
                    let result = api_client.get_ai_session_outputs(work_id).await;
                    let mut ai_session_outputs_result = outputs_result_clone.lock().unwrap();
                    *ai_session_outputs_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut ai_session_outputs_result = outputs_result_clone.lock().unwrap();
                    *ai_session_outputs_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_project_details(&self, project_id: i64, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            tracing::info!(
                "refresh_project_details called for project_id={}",
                project_id
            );
            state.loading_project_details = true;
            state.project_details_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.project_details_result);

            tokio::spawn(async move {
                tracing::info!("Fetching project details for project_id={}", project_id);
                if let Some(api_client) = connection_manager.get_api_client().await {
                    let result = api_client.get_project_details(project_id).await;
                    match &result {
                        Ok(details) => tracing::info!(
                            "Project details loaded: {} components",
                            details.components.len()
                        ),
                        Err(e) => tracing::error!("Failed to load project details: {}", e),
                    }
                    let mut project_details_result = result_clone.lock().unwrap();
                    *project_details_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    tracing::error!("No API client available for project details");
                    let mut project_details_result = result_clone.lock().unwrap();
                    *project_details_result = Some(Err("Not connected".to_string()));
                }
            });
        } else {
            tracing::warn!("refresh_project_details called but not connected");
        }
    }

    // Local server methods
    pub async fn check_local_server(&self, state: &mut AppState) -> Result<(), String> {
        state.ui_state.checking_local_server = true;
        // This will be implemented with actual API calls
        Ok(())
    }
}
