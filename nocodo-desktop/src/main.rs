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
    let api_url_clone1 = api_url.clone();
    let api_url_clone2 = api_url.clone();

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
