slint::include_modules!();

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
    let api_url_clone1 = api_url.clone();
    let api_url_clone2 = api_url.clone();
    let api_url_clone3 = api_url.clone();
    let api_url_clone4 = api_url.clone();

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
