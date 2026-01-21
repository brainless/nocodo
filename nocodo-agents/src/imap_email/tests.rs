use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn create_test_agent() -> ImapEmailAgent {
        use crate::database::Database;
        use nocodo_llm_sdk::claude::ClaudeClient;
        use nocodo_tools::ToolExecutor;
        use std::sync::Arc;

        let _ = std::env::var("ANTHROPIC_API_KEY").ok();

        let _temp_db = NamedTempFile::new().unwrap();
        let _db_path = _temp_db.path().to_str().unwrap().to_string();

        let client = Arc::new(
            ClaudeClient::new("test-key".to_string())
                .unwrap()
                .with_base_url("https://api.anthropic.com".to_string()),
        );

        let database = Arc::new(Database::new(&PathBuf::from(":memory:")).unwrap());
        let tool_executor = Arc::new(ToolExecutor::new(PathBuf::from(".")));

        ImapEmailAgent::new(
            client,
            database,
            tool_executor,
            "imap.example.com".to_string(),
            993,
            "user@example.com".to_string(),
            "password123".to_string(),
        )
    }

    #[test]
    fn test_agent_settings_schema() {
        let schema = ImapEmailAgent::static_settings_schema().unwrap();
        assert_eq!(schema.agent_name, "IMAP Email Agent");
        assert_eq!(schema.section_name, "imap_email");
        assert_eq!(schema.settings.len(), 4);

        let host_field = schema.settings.iter().find(|s| s.name == "host").unwrap();
        assert_eq!(host_field.label, "IMAP Server");
        assert!(host_field.required);

        let port_field = schema.settings.iter().find(|s| s.name == "port").unwrap();
        assert_eq!(port_field.default_value, Some("993".to_string()));
        assert!(!port_field.required);

        let username_field = schema
            .settings
            .iter()
            .find(|s| s.name == "username")
            .unwrap();
        assert_eq!(username_field.label, "Email Address");
        assert!(username_field.required);

        let password_field = schema
            .settings
            .iter()
            .find(|s| s.name == "password")
            .unwrap();
        assert_eq!(password_field.setting_type, SettingType::Password);
        assert!(password_field.required);
    }

    #[test]
    fn test_agent_tools() {
        let agent = create_test_agent();
        let tools = agent.tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0], AgentTool::ImapReader);
    }

    #[test]
    fn test_agent_objective() {
        let agent = create_test_agent();
        assert_eq!(agent.objective(), "Analyze and manage emails via IMAP");
    }

    #[test]
    fn test_system_prompt() {
        let agent = create_test_agent();
        let prompt = agent.system_prompt();
        assert!(prompt.contains("email analysis expert"));
        assert!(prompt.contains("list_mailboxes"));
        assert!(prompt.contains("search"));
        assert!(prompt.contains("fetch_headers"));
        assert!(prompt.contains("fetch_email"));
    }

    #[test]
    fn test_from_settings_missing_required() {
        use crate::database::Database;
        use nocodo_llm_sdk::claude::ClaudeClient;
        use nocodo_tools::ToolExecutor;
        use std::collections::HashMap;
        use std::sync::Arc;

        let mut settings = HashMap::new();
        settings.insert("host".to_string(), "imap.gmail.com".to_string());
        settings.insert("port".to_string(), "993".to_string());
        settings.insert("username".to_string(), "user@gmail.com".to_string());

        let client = Arc::new(
            ClaudeClient::new("test-key".to_string())
                .unwrap()
                .with_base_url("https://api.anthropic.com".to_string()),
        );

        let database = Arc::new(Database::new(&PathBuf::from(":memory:")).unwrap());
        let tool_executor = Arc::new(ToolExecutor::new(PathBuf::from(".")));

        let agent = ImapEmailAgent::from_settings(client, database, tool_executor, &settings);

        assert!(agent.is_err());
        match agent {
            Ok(_) => panic!("Should fail without password"),
            Err(e) => assert!(e.to_string().contains("password")),
        }
    }

    #[test]
    fn test_from_settings_default_port() {
        use crate::database::Database;
        use nocodo_llm_sdk::claude::ClaudeClient;
        use nocodo_tools::ToolExecutor;
        use std::collections::HashMap;
        use std::sync::Arc;

        let mut settings = HashMap::new();
        settings.insert("host".to_string(), "imap.gmail.com".to_string());
        settings.insert("username".to_string(), "user@gmail.com".to_string());
        settings.insert("password".to_string(), "app-password".to_string());

        let client = Arc::new(
            ClaudeClient::new("test-key".to_string())
                .unwrap()
                .with_base_url("https://api.anthropic.com".to_string()),
        );

        let database = Arc::new(Database::new(&PathBuf::from(":memory:")).unwrap());
        let tool_executor = Arc::new(ToolExecutor::new(PathBuf::from(".")));

        let agent = ImapEmailAgent::from_settings(client, database, tool_executor, &settings);

        assert!(agent.is_ok());
    }
}
