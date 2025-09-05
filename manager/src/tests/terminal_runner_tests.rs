#[cfg(test)]
mod tests {
    use crate::terminal_runner::TerminalRunner;
    use crate::database::Database;
    use crate::models::TerminalSession;
    use crate::websocket::{WebSocketBroadcaster, WebSocketServer};
    use actix::Actor;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_terminal_runner_creation() {
        // Create temporary database
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database = Arc::new(Database::new(&db_path).unwrap());

        // Create WebSocket server
        let ws_server = WebSocketServer::default().start();
        let broadcaster = Arc::new(WebSocketBroadcaster::new(ws_server));

        // Create terminal runner
        let terminal_runner = TerminalRunner::new(database, broadcaster);

        // Check that default tools are registered
        let tools = terminal_runner.get_tool_registry().await;
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "claude"));
        assert!(tools.iter().any(|t| t.name == "gemini"));
        assert!(tools.iter().any(|t| t.name == "qwen"));
    }

    #[tokio::test]
    async fn test_terminal_session_creation() {
        let session = TerminalSession::new(
            "work-1".to_string(),
            "message-1".to_string(),
            "claude".to_string(),
            Some("Test project context".to_string()),
            true,
            true,
            80,
            24,
        );

        assert_eq!(session.tool_name, "claude");
        assert_eq!(session.requires_pty, true);
        assert_eq!(session.interactive, true);
        assert_eq!(session.cols, 80);
        assert_eq!(session.rows, 24);
        assert_eq!(session.status, "running");
        assert!(session.project_context.is_some());
    }
}