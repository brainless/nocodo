use serde::Serialize;
use std::{env, fs, io, path::Path};

const DEFAULT_API_HOST: &str = "127.0.0.1";
const DEFAULT_API_PORT: u16 = 8080;
const DEFAULT_DB_KIND: &str = "sqlite";
const DEFAULT_DB_PATH: &str = "app.db";

#[derive(Debug, Clone, Serialize)]
pub struct EffectiveConfig {
    pub project_name: Option<String>,
    pub db_kind: String,
    pub database_url: String,
    pub api_host: String,
    pub api_port: u16,
}

impl EffectiveConfig {
    pub fn api_bind_addr(&self) -> String {
        format!("{}:{}", self.api_host, self.api_port)
    }
}

#[derive(Debug, Clone)]
pub struct ConfigOptions {
    pub project_conf_path: String,
}

impl Default for ConfigOptions {
    fn default() -> Self {
        Self {
            project_conf_path: "project.conf".to_string(),
        }
    }
}

pub fn load(options: &ConfigOptions) -> io::Result<EffectiveConfig> {
    let project_conf_vars = read_project_conf(&options.project_conf_path)?;

    let project_name = project_conf_vars.get("PROJECT_NAME").cloned();

    let db_kind = env::var("DB_KIND")
        .ok()
        .or_else(|| project_conf_vars.get("DB_KIND").cloned())
        .unwrap_or_else(|| DEFAULT_DB_KIND.to_string());

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());

    let api_host = env::var("NOCODO_API_HOST").unwrap_or_else(|_| DEFAULT_API_HOST.to_string());

    let api_port = env::var("NOCODO_API_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_API_PORT);

    Ok(EffectiveConfig {
        project_name,
        db_kind,
        database_url,
        api_host,
        api_port,
    })
}

fn read_project_conf(path: &str) -> io::Result<std::collections::HashMap<String, String>> {
    if !Path::new(path).exists() {
        return Ok(Default::default());
    }

    let content = fs::read_to_string(path)?;
    let mut vars = std::collections::HashMap::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };

        let key = raw_key.trim();
        if key.is_empty() {
            continue;
        }

        let value = unquote(raw_value.trim());
        vars.insert(key.to_string(), value.to_string());
    }

    Ok(vars)
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_surrounding_quotes() {
        assert_eq!(unquote("\"nocodo\""), "nocodo");
        assert_eq!(unquote("nocodo"), "nocodo");
    }
}
