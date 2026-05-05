use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub project: ProjectConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub gui: GuiConfig,
    pub admin_gui: AdminGuiConfig,
    #[serde(default)]
    pub projects: Option<ProjectsConfig>,
    #[serde(default)]
    pub auth: Option<AuthSettings>,
    #[serde(default)]
    pub agents: Option<AgentsConfig>,
    #[serde(default)]
    pub pm_agent: Option<PmAgentConfig>,
    #[serde(default)]
    pub api_keys: Option<ApiKeysConfig>,
    #[serde(default)]
    pub deploy: Option<DeployConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub title: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub kind: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GuiConfig {
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminGuiConfig {
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProjectsConfig {
    pub default_path: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthSettings {
    #[serde(default = "default_true")]
    pub mandatory: bool,
    pub resend_api_key: Option<String>,
    pub from_email: Option<String>,
}

fn default_true() -> bool { true }

#[derive(Debug, Deserialize, Clone)]
pub struct AgentsConfig {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PmAgentConfig {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiKeysConfig {
    pub openai_api_key: Option<String>,
    pub groq_api_key: Option<String>,
    pub cerebras_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeployConfig {
    pub server_ip: String,
    pub ssh_user: String,
    pub domain_name: String,
    pub letsencrypt_email: Option<String>,
    pub remote_base_dir: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = Self::find_config_file().ok_or_else(|| {
            "project.toml not found. Copy project.toml.template to project.toml and fill in your values.".to_string()
        })?;

        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let mut config: Config = toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        config.apply_env_overrides();
        Ok(config)
    }

    /// Export config values to env vars so that crates that read env vars directly
    /// (e.g. nocodo-agents) work without manual `source project.toml` in local dev.
    /// Only sets vars that are not already present in the environment.
    pub fn export_to_env(&self) {
        if let Some(keys) = &self.api_keys {
            set_env_if_unset("OPENAI_API_KEY", keys.openai_api_key.as_deref());
            set_env_if_unset("GROQ_API_KEY", keys.groq_api_key.as_deref());
            set_env_if_unset("CEREBRAS_API_KEY", keys.cerebras_api_key.as_deref());
            set_env_if_unset("ANTHROPIC_API_KEY", keys.anthropic_api_key.as_deref());
        }
        if let Some(agents) = &self.agents {
            set_env_if_unset("AGENT_PROVIDER", Some(agents.provider.as_str()));
            set_env_if_unset("AGENT_MODEL", Some(agents.model.as_str()));
        }
        if let Some(pm) = &self.pm_agent {
            set_env_if_unset("PM_AGENT_PROVIDER", Some(pm.provider.as_str()));
            set_env_if_unset("PM_AGENT_MODEL", Some(pm.model.as_str()));
        }
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("BACKEND_HOST") { self.server.host = v; }
        if let Ok(v) = std::env::var("BACKEND_PORT") {
            if let Ok(p) = v.parse() { self.server.port = p; }
        }
        if let Ok(v) = std::env::var("DATABASE_URL") { self.database.url = v; }
        if let Ok(v) = std::env::var("GUI_PORT") {
            if let Ok(p) = v.parse() { self.gui.port = p; }
        }
        if let Ok(v) = std::env::var("ADMIN_GUI_PORT") {
            if let Ok(p) = v.parse() { self.admin_gui.port = p; }
        }
        if let Ok(v) = std::env::var("DEFAULT_PROJECTS_PATH") {
            self.projects.get_or_insert_with(|| ProjectsConfig { default_path: None }).default_path = Some(v);
        }
        if let Ok(v) = std::env::var("MANDATORY_AUTHENTICATION") {
            let b = !matches!(v.to_lowercase().as_str(), "false" | "0" | "no");
            self.auth.get_or_insert_with(|| AuthSettings { mandatory: true, resend_api_key: None, from_email: None }).mandatory = b;
        }
        if let Ok(v) = std::env::var("RESEND_API_KEY") {
            self.auth.get_or_insert_with(|| AuthSettings { mandatory: true, resend_api_key: None, from_email: None }).resend_api_key = Some(v);
        }
        if let Ok(v) = std::env::var("AUTH_FROM_EMAIL") {
            self.auth.get_or_insert_with(|| AuthSettings { mandatory: true, resend_api_key: None, from_email: None }).from_email = Some(v);
        }
    }

    pub fn find_config_file() -> Option<PathBuf> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));

        let mut candidates = vec![
            PathBuf::from("project.toml"),
            PathBuf::from("../project.toml"),
        ];

        if let Some(ref dir) = exe_dir {
            candidates.push(dir.join("../../project.toml"));
            candidates.push(dir.join("project.toml"));
        }

        candidates.into_iter().find(|p| p.exists())
    }
}

fn set_env_if_unset(key: &str, val: Option<&str>) {
    if let Some(v) = val {
        if !v.is_empty() && std::env::var(key).is_err() {
            std::env::set_var(key, v);
        }
    }
}

/// Compat shim for handler code that calls read_project_conf() directly.
/// Loads config on each call — acceptable since it's a file read only.
pub fn read_project_conf(key: &str) -> Option<String> {
    let config = Config::load().ok()?;
    match key {
        "DATABASE_URL"         => Some(config.database.url),
        "BACKEND_HOST"         => Some(config.server.host),
        "BACKEND_PORT"         => Some(config.server.port.to_string()),
        "GUI_PORT"             => Some(config.gui.port.to_string()),
        "ADMIN_GUI_PORT"       => Some(config.admin_gui.port.to_string()),
        "DOMAIN_NAME"          => config.deploy.as_ref().map(|d| d.domain_name.clone()),
        "DEFAULT_PROJECTS_PATH" => config.projects.as_ref().and_then(|p| p.default_path.clone()),
        "MANDATORY_AUTHENTICATION" => config.auth.as_ref().map(|a| a.mandatory.to_string()),
        "RESEND_API_KEY"       => config.auth.as_ref().and_then(|a| a.resend_api_key.clone()),
        "AUTH_FROM_EMAIL"      => config.auth.as_ref().and_then(|a| a.from_email.clone()),
        "AGENT_PROVIDER"       => config.agents.as_ref().map(|a| a.provider.clone()),
        "AGENT_MODEL"          => config.agents.as_ref().map(|a| a.model.clone()),
        "PM_AGENT_PROVIDER"    => config.pm_agent.as_ref().map(|a| a.provider.clone()),
        "PM_AGENT_MODEL"       => config.pm_agent.as_ref().map(|a| a.model.clone()),
        "OPENAI_API_KEY"       => config.api_keys.as_ref().and_then(|k| k.openai_api_key.clone()),
        "GROQ_API_KEY"         => config.api_keys.as_ref().and_then(|k| k.groq_api_key.clone()),
        "CEREBRAS_API_KEY"     => config.api_keys.as_ref().and_then(|k| k.cerebras_api_key.clone()),
        "ANTHROPIC_API_KEY"    => config.api_keys.as_ref().and_then(|k| k.anthropic_api_key.clone()),
        _ => None,
    }
}

/// Returns the resolved config file path for diagnostic logging.
pub fn resolved_config_path() -> Option<PathBuf> {
    Config::find_config_file()
}
