slint::include_modules!();

use slint::Model;
use std::env;

const DEFAULT_API_URL: &str = "http://127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_url = env::var("NOCODO_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    println!("Connecting to API at: {}", api_url);

    let ui = AppWindow::new()?;

    let ui_weak = ui.as_weak();
    let ui_weak2 = ui_weak.clone();
    let ui_weak3 = ui_weak.clone();
    let ui_weak4 = ui_weak.clone();
    let ui_weak5 = ui_weak.clone();
    let ui_weak6 = ui_weak.clone();
    let ui_weak7 = ui_weak.clone();
    let api_url_clone1 = api_url.clone();
    let api_url_clone2 = api_url.clone();
    let api_url_clone3 = api_url.clone();
    let api_url_clone4 = api_url.clone();
    let api_url_clone5 = api_url.clone();
    let api_url_clone6 = api_url.clone();
    let api_url_clone7 = api_url.clone();

    ui.on_load_settings(move || {
        let ui = ui_weak.upgrade().unwrap();
        let api_url = api_url_clone1.clone();

        slint::spawn_local(async move {
            match fetch_settings(&api_url).await {
                Ok(settings) => {
                    let entries: Vec<ApiKeyEntry> = settings
                        .api_keys
                        .iter()
                        .map(|api_key| ApiKeyEntry {
                            name: api_key.name.clone().into(),
                            key: api_key
                                .key
                                .as_ref()
                                .unwrap_or(&"".to_string())
                                .clone()
                                .into(),
                        })
                        .collect();

                    ui.set_api_keys(slint::ModelRc::new(slint::VecModel::from(entries)));
                }
                Err(e) => {
                    eprintln!("Failed to fetch settings: {}", e);
                }
            }
        })
        .unwrap();
    });

    ui.on_load_agents(move || {
        let ui = ui_weak3.upgrade().unwrap();
        let api_url = api_url_clone3.clone();

        slint::spawn_local(async move {
            match fetch_agents(&api_url).await {
                Ok(agents_response) => {
                    let enabled_agents: Vec<&shared_types::AgentInfo> = agents_response
                        .agents
                        .iter()
                        .filter(|agent| agent.enabled)
                        .collect();

                    let entries: Vec<AgentEntry> = enabled_agents
                        .iter()
                        .map(|agent| AgentEntry {
                            id: agent.id.clone().into(),
                            name: agent.name.clone().into(),
                        })
                        .collect();

                    let names: Vec<slint::SharedString> = enabled_agents
                        .iter()
                        .map(|agent| agent.name.clone().into())
                        .collect();

                    ui.set_agents(slint::ModelRc::new(slint::VecModel::from(entries)));
                    ui.set_agent_names(slint::ModelRc::new(slint::VecModel::from(names)));
                }
                Err(e) => {
                    eprintln!("Failed to fetch agents: {}", e);
                }
            }
        })
        .unwrap();
    });

    ui.on_load_chats(move || {
        let ui = ui_weak6.upgrade().unwrap();
        let api_url = api_url_clone6.clone();

        slint::spawn_local(async move {
            match fetch_sessions(&api_url).await {
                Ok(sessions_response) => {
                    let entries: Vec<ChatListItem> = sessions_response
                        .sessions
                        .iter()
                        .map(|session| ChatListItem {
                            id: session.id as i32,
                            agent_name: session.agent_name.clone().into(),
                            user_prompt: session.user_prompt.clone().into(),
                            created_at: session.created_at.clone().into(),
                        })
                        .collect();

                    ui.set_chat_list(slint::ModelRc::new(slint::VecModel::from(entries)));
                }
                Err(e) => {
                    eprintln!("Failed to fetch sessions: {}", e);
                }
            }
        })
        .unwrap();
    });

    ui.on_handle_start_agent(move || {
        let ui = ui_weak5.upgrade().unwrap();
        let api_url = api_url_clone5.clone();

        slint::spawn_local(async move {
            if let Err(e) = handle_agent_start(&ui, &api_url).await {
                eprintln!("Failed to start agent: {}", e);
            }
        })
        .unwrap();
    });

    slint::spawn_local(async move {
        match fetch_settings(&api_url_clone2).await {
            Ok(settings) => {
                let ui = ui_weak2.upgrade().unwrap();
                let entries: Vec<ApiKeyEntry> = settings
                    .api_keys
                    .iter()
                    .map(|api_key| ApiKeyEntry {
                        name: api_key.name.clone().into(),
                        key: api_key
                            .key
                            .as_ref()
                            .unwrap_or(&"".to_string())
                            .clone()
                            .into(),
                    })
                    .collect();

                ui.set_api_keys(slint::ModelRc::new(slint::VecModel::from(entries)));
            }
            Err(e) => {
                eprintln!("Failed to fetch settings: {}", e);
            }
        }
    })
    .unwrap();

    slint::spawn_local(async move {
        match fetch_agents(&api_url_clone4).await {
            Ok(agents_response) => {
                let ui = ui_weak4.upgrade().unwrap();
                let enabled_agents: Vec<&shared_types::AgentInfo> = agents_response
                    .agents
                    .iter()
                    .filter(|agent| agent.enabled)
                    .collect();

                let entries: Vec<AgentEntry> = enabled_agents
                    .iter()
                    .map(|agent| AgentEntry {
                        id: agent.id.clone().into(),
                        name: agent.name.clone().into(),
                    })
                    .collect();

                let names: Vec<slint::SharedString> = enabled_agents
                    .iter()
                    .map(|agent| agent.name.clone().into())
                    .collect();

                ui.set_agents(slint::ModelRc::new(slint::VecModel::from(entries)));
                ui.set_agent_names(slint::ModelRc::new(slint::VecModel::from(names)));
            }
            Err(e) => {
                eprintln!("Failed to fetch agents: {}", e);
            }
        }
    })
    .unwrap();

    slint::spawn_local(async move {
        match fetch_sessions(&api_url_clone7).await {
            Ok(sessions_response) => {
                let ui = ui_weak7.upgrade().unwrap();
                let entries: Vec<ChatListItem> = sessions_response
                    .sessions
                    .iter()
                    .map(|session| ChatListItem {
                        id: session.id as i32,
                        agent_name: session.agent_name.clone().into(),
                        user_prompt: session.user_prompt.clone().into(),
                        created_at: session.created_at.clone().into(),
                    })
                    .collect();

                ui.set_chat_list(slint::ModelRc::new(slint::VecModel::from(entries)));
            }
            Err(e) => {
                eprintln!("Failed to fetch sessions: {}", e);
            }
        }
    })
    .unwrap();

    Ok(ui.run()?)
}

async fn fetch_settings(
    api_url: &str,
) -> Result<shared_types::SettingsResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client.get(&format!("{}/settings", api_url)).send().await?;

    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let settings: shared_types::SettingsResponse = response.json().await?;
    Ok(settings)
}

async fn fetch_agents(
    api_url: &str,
) -> Result<shared_types::AgentsResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client.get(&format!("{}/agents", api_url)).send().await?;

    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let agents: shared_types::AgentsResponse = response.json().await?;
    Ok(agents)
}

async fn fetch_sessions(
    api_url: &str,
) -> Result<shared_types::SessionListResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/agents/sessions", api_url))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let sessions: shared_types::SessionListResponse = response.json().await?;
    Ok(sessions)
}

fn build_agent_config(
    agent_id: &str,
) -> Result<shared_types::AgentConfig, Box<dyn std::error::Error>> {
    match agent_id {
        "sqlite" => {
            // Hardcoded default: use hackernews database in user's home
            let home_dir = env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            let db_path = format!("{}/.local/share/nocodo/hackernews.db", home_dir);

            Ok(shared_types::AgentConfig::Sqlite(
                shared_types::SqliteAgentConfig { db_path },
            ))
        }
        "codebase-analysis" => {
            // Hardcoded default: current directory
            let path = env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());

            Ok(shared_types::AgentConfig::CodebaseAnalysis(
                shared_types::CodebaseAnalysisAgentConfig {
                    path,
                    max_depth: Some(3),
                },
            ))
        }
        _ => Err(format!("Unknown agent: {}", agent_id).into()),
    }
}

async fn execute_agent(
    api_url: &str,
    agent_id: &str,
    user_prompt: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // Build config based on agent_id
    let config = build_agent_config(agent_id)?;

    let request = shared_types::AgentExecutionRequest {
        user_prompt: user_prompt.to_string(),
        config,
    };

    // Determine endpoint based on agent_id
    let endpoint = match agent_id {
        "sqlite" => format!("{}/agents/sqlite/execute", api_url),
        "codebase-analysis" => format!("{}/agents/codebase-analysis/execute", api_url),
        _ => return Err(format!("Unknown agent: {}", agent_id).into()),
    };

    let response = client.post(&endpoint).json(&request).send().await?;

    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let execution_response: shared_types::AgentExecutionResponse = response.json().await?;

    Ok(execution_response.session_id)
}

async fn fetch_session(
    api_url: &str,
    session_id: i64,
) -> Result<shared_types::SessionResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/agents/sessions/{}", api_url, session_id))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let session: shared_types::SessionResponse = response.json().await?;
    Ok(session)
}

async fn handle_agent_start(
    ui: &AppWindow,
    api_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Get selected agent index and input text
    let selected_index = ui.get_chats_selected_index();
    let input_text = ui.get_input_text();

    // Validate selection
    if selected_index < 0 {
        return Err("No agent selected".into());
    }

    if input_text.is_empty() {
        return Err("Input text is required".into());
    }

    // 2. Get agent info from agents array
    let agents = ui.get_agents();
    if selected_index as usize >= agents.row_count() {
        return Err("Invalid agent selection".into());
    }

    let agent_entry = agents.row_data(selected_index as usize).unwrap();
    let agent_id = agent_entry.id.to_string();
    let agent_name = agent_entry.name.to_string();

    // 3. Navigate to chat detail page immediately with loading state
    ui.set_current_page("chat-detail".into());
    ui.set_is_loading_chat(true);
    ui.set_current_session_agent_name(agent_name.clone().into());
    ui.set_chat_messages(slint::ModelRc::new(slint::VecModel::from(vec![])));

    // 4. Execute agent (this will be async)
    let session_id = execute_agent(api_url, &agent_id, &input_text).await?;

    // 5. Fetch session data
    let session = fetch_session(api_url, session_id).await?;

    // 6. Transform messages to ChatMessage format
    let chat_messages: Vec<ChatMessage> = session
        .messages
        .iter()
        .map(|msg| ChatMessage {
            role: msg.role.clone().into(),
            content: msg.content.clone().into(),
        })
        .collect();

    // 7. Update UI
    ui.set_is_loading_chat(false);
    ui.set_chat_messages(slint::ModelRc::new(slint::VecModel::from(chat_messages)));

    Ok(())
}
