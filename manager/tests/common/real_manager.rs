use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

use crate::common::{config::TestConfig, llm_config::LlmProviderTestConfig};

/// RealManagerInstance manages a real nocodo-manager daemon for E2E testing
pub struct RealManagerInstance {
    pub process: Child,
    pub base_url: String,
    pub port: u16,
    pub config: TestConfig,
    pub config_file_path: PathBuf,
}

impl RealManagerInstance {
    /// Start a real nocodo-manager instance with test configuration
    pub async fn start(llm_provider: &LlmProviderTestConfig) -> anyhow::Result<Self> {
        let config = TestConfig::new();
        let port = Self::find_free_port()?;
        let base_url = format!("http://127.0.0.1:{}", port);

        // Create manager configuration file
        let config_file_path = config.temp_dir_path().join("manager.toml");
        let toml_config = Self::create_manager_config(&config, port, llm_provider)?;
        std::fs::write(&config_file_path, toml_config)?;

        tracing::info!(
            "Starting real manager instance on port {} with config: {}",
            port,
            config_file_path.display()
        );

        // Build the manager binary if not already built
        let cargo_build = Command::new("cargo")
            .args(["build", "--bin", "nocodo-manager"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()?;

        if !cargo_build.success() {
            return Err(anyhow::anyhow!("Failed to build nocodo-manager binary"));
        }

        // Start the manager daemon
        tracing::info!("Starting manager with config: {}", config_file_path.display());
        let mut process = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "nocodo-manager",
                "--",
                "--config",
                config_file_path.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Check if process started successfully
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Ok(Some(status)) = process.try_wait() {
            let stdout = process.stdout.take();
            let stderr = process.stderr.take();

            if let Some(mut stdout) = stdout {
                let mut output = String::new();
                std::io::Read::read_to_string(&mut stdout, &mut output).ok();
                tracing::error!("Manager stdout: {}", output);
            }

            if let Some(mut stderr) = stderr {
                let mut output = String::new();
                std::io::Read::read_to_string(&mut stderr, &mut output).ok();
                tracing::error!("Manager stderr: {}", output);
            }

            return Err(anyhow::anyhow!("Manager process exited with status: {}", status));
        }

        let mut instance = RealManagerInstance {
            process,
            base_url: base_url.clone(),
            port,
            config,
            config_file_path,
        };

        // Wait for the server to be ready
        instance.wait_for_ready().await?;

        tracing::info!("Real manager instance started successfully at {}", base_url);
        Ok(instance)
    }

    /// Find a free port for the manager instance
    fn find_free_port() -> anyhow::Result<u16> {
        // Try random ports in the high range to avoid conflicts
        for _ in 0..10 {
            let port = 8000 + (rand::random::<u16>() % 2000); // 8000-9999
            if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
                drop(listener);
                return Ok(port);
            }
        }
        Err(anyhow::anyhow!("Could not find a free port"))
    }

    /// Create manager configuration TOML
    fn create_manager_config(
        config: &TestConfig,
        port: u16,
        llm_provider: &LlmProviderTestConfig,
    ) -> anyhow::Result<String> {
        let api_key = std::env::var(&llm_provider.api_key_env)
            .map_err(|_| anyhow::anyhow!("API key not found for provider: {}", llm_provider.name))?;

        let config_toml = format!(
            r#"[server]
host = "127.0.0.1"
port = {}

[database]
path = "{}"

[socket]
path = "{}"

[api_keys]
{}
"#,
            port,
            config.db_path().display(),
            config.temp_dir_path().join("test.sock").display(),
            match llm_provider.name.as_str() {
                "grok" => format!("grok_api_key = \"{}\"", api_key),
                "openai" => format!("openai_api_key = \"{}\"", api_key),
                "anthropic" => format!("anthropic_api_key = \"{}\"", api_key),
                _ => return Err(anyhow::anyhow!("Unknown provider: {}", llm_provider.name)),
            }
        );

        Ok(config_toml)
    }

    /// Wait for the manager to be ready by checking health endpoint
    async fn wait_for_ready(&mut self) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let health_url = format!("{}/api/health", self.base_url);

        for attempt in 1..=30 {
            // Check if process is still running
            if let Ok(Some(_)) = self.process.try_wait() {
                return Err(anyhow::anyhow!("Manager process exited unexpectedly"));
            }

            match client.get(&health_url).send().await {
                Ok(response) if response.status().is_success() => {
                    tracing::info!("Manager ready after {} attempts", attempt);
                    return Ok(());
                }
                _ => {
                    if attempt <= 30 {
                        sleep(Duration::from_millis(1000)).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Manager failed to become ready within 30 seconds"))
    }

    /// Create an HTTP client for making API calls
    pub fn http_client(&self) -> reqwest::Client {
        reqwest::Client::new()
    }

    /// Get the base URL for API calls
    pub fn api_url(&self, path: &str) -> String {
        format!("{}/api{}", self.base_url, path)
    }

    /// Create a project using the real API
    pub async fn create_project(&self, name: &str, path: &str) -> anyhow::Result<String> {
        let client = self.http_client();
        let response = client
            .post(&self.api_url("/projects"))
            .json(&serde_json::json!({
                "name": name,
                "path": path,
                "language": null,
                "framework": null,
                "template": null
            }))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error body".to_string());
            return Err(anyhow::anyhow!(
                "Failed to create project: {} - {}",
                status,
                error_body
            ));
        }

        let body: serde_json::Value = response.json().await?;
        let project_id = body["project"]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No project ID in response"))?;

        Ok(project_id.to_string())
    }

    /// Create work using the real API
    pub async fn create_work(&self, title: &str, project_id: Option<String>) -> anyhow::Result<String> {
        let client = self.http_client();
        let response = client
            .post(&self.api_url("/work"))
            .json(&serde_json::json!({
                "title": title,
                "project_id": project_id
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to create work: {}", response.status()));
        }

        let body: serde_json::Value = response.json().await?;
        let work_id = body["work"]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No work ID in response"))?;

        Ok(work_id.to_string())
    }

    /// Add message to work using the real API
    pub async fn add_message(&self, work_id: &str, content: &str) -> anyhow::Result<String> {
        let client = self.http_client();
        let response = client
            .post(&self.api_url(&format!("/work/{}/messages", work_id)))
            .json(&serde_json::json!({
                "content": content,
                "content_type": "text",
                "author_type": "user"
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to add message: {}",
                response.status()
            ));
        }

        let body: serde_json::Value = response.json().await?;
        let message_id = body["message"]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No message ID in response"))?;

        Ok(message_id.to_string())
    }

    /// Create AI session using the real API
    pub async fn create_ai_session(&self, work_id: &str, message_id: &str) -> anyhow::Result<String> {
        let client = self.http_client();
        let response = client
            .post(&self.api_url(&format!("/work/{}/sessions", work_id)))
            .json(&serde_json::json!({
                "message_id": message_id,
                "tool_name": "llm-agent"
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to create AI session: {}",
                response.status()
            ));
        }

        let body: serde_json::Value = response.json().await?;
        let session_id = body["session"]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No session ID in response"))?;

        Ok(session_id.to_string())
    }

    /// Get AI session outputs using the real API
    pub async fn get_ai_outputs(&self, work_id: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let client = self.http_client();
        let response = client
            .get(&self.api_url(&format!("/work/{}/outputs", work_id)))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get AI outputs: {}",
                response.status()
            ));
        }

        let body: serde_json::Value = response.json().await?;
        let outputs = body["outputs"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No outputs array in response"))?;

        Ok(outputs.clone())
    }
}

impl Drop for RealManagerInstance {
    fn drop(&mut self) {
        // Terminate the manager process
        if let Err(e) = self.process.kill() {
            tracing::warn!("Failed to kill manager process: {}", e);
        }

        // Clean up the configuration file
        if let Err(e) = std::fs::remove_file(&self.config_file_path) {
            tracing::warn!("Failed to remove config file: {}", e);
        }

        tracing::info!("Real manager instance cleaned up");
    }
}