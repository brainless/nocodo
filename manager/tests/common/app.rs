use actix::Actor;
use actix_web::{test, web, App};
use std::sync::Arc;
use std::time::SystemTime;

use nocodo_manager::database::Database;
use nocodo_manager::handlers::AppState;
use nocodo_manager::routes::configure_routes;
use nocodo_manager::websocket::{WebSocketBroadcaster, WebSocketServer};

use super::config::TestConfig;
use super::database::TestDatabase;
use super::llm_config::LlmProviderTestConfig;

use nocodo_manager::llm_agent::LlmAgent;

/// TestApp provides a fully configured test application with isolated resources
pub struct TestApp {
    pub config: TestConfig,
    pub database: TestDatabase,
    pub app_state: web::Data<AppState>,
}

#[allow(dead_code)]
impl TestApp {
    /// Create a new isolated test application
    pub async fn new() -> Self {
        let config = TestConfig::new();
        let database = TestDatabase::new().unwrap();

        // Create WebSocket server and broadcaster
        let ws_server = WebSocketServer::default().start();
        let ws_broadcaster = Arc::new(WebSocketBroadcaster::new(ws_server));

        // Create app state
        let app_state = web::Data::new(AppState {
            database: database.database.clone(),
            start_time: SystemTime::now(),
            ws_broadcaster,
            llm_agent: None, // Not needed for basic API tests
            config: Arc::new(std::sync::RwLock::new(config.config.clone())),
        });

        // Create the test service with all routes
        let _test_service = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .configure(|cfg| configure_routes(cfg, false)),
        )
        .await;

        Self {
            config,
            database,
            app_state,
        }
    }

    /// Get the app state
    pub fn app_state(&self) -> &web::Data<AppState> {
        &self.app_state
    }

    /// Get the database
    pub fn db(&self) -> &Arc<Database> {
        &self.database.database
    }

    /// Get the test configuration
    pub fn test_config(&self) -> &TestConfig {
        &self.config
    }

    /// Create a new test application with real LLM integration
    pub async fn new_with_llm(provider: &LlmProviderTestConfig) -> Self {
        let config = TestConfig::new();
        let database = TestDatabase::new().unwrap();

        // Create WebSocket server and broadcaster
        let ws_server = WebSocketServer::default().start();
        let ws_broadcaster = Arc::new(WebSocketBroadcaster::new(ws_server));

        // Create real LLM agent with provider configuration
        let llm_agent = Some(Arc::new(LlmAgent::new(
            database.database.clone(),
            ws_broadcaster.clone(),
            config.projects_dir(),
            Arc::new(provider.to_app_config()),
        )));

        // Create app state with LLM agent
        let app_state = web::Data::new(AppState {
            database: database.database.clone(),
            start_time: SystemTime::now(),
            ws_broadcaster,
            llm_agent, // Real LLM agent, not None!
            config: Arc::new(std::sync::RwLock::new(config.config.clone())),
        });

        // Create the test service with all routes
        let _test_service = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .configure(|cfg| configure_routes(cfg, false)),
        )
        .await;

        Self {
            config,
            database,
            app_state,
        }
    }

    /// Send a message to the LLM agent and get the response
    pub async fn send_llm_message(
        &self,
        session_id: i64,
        message: String,
    ) -> anyhow::Result<String> {
        if let Some(llm_agent) = &self.app_state.llm_agent {
            llm_agent.process_message(session_id, message).await
        } else {
            Err(anyhow::anyhow!("No LLM agent configured for this test app"))
        }
    }

    /// Get the LLM agent if available
    pub fn llm_agent(&self) -> anyhow::Result<&Arc<LlmAgent>> {
        self.app_state
            .llm_agent
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No LLM agent configured"))
    }

    /// Clean up resources (called automatically on drop)
    pub fn cleanup(&self) {
        // Cleanup is handled by the TestDatabase and TestConfig
        tracing::debug!("TestApp cleanup: {}", self.config.test_id);
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_rt::test]
    async fn test_test_app_creation() {
        let test_app = TestApp::new().await;

        // Test health check endpoint
        let req = test::TestRequest::get().uri("/api/health").to_request();
        let service = test::init_service(
            App::new()
                .app_data(test_app.app_state.clone())
                .configure(|cfg| configure_routes(cfg, false)),
        )
        .await;
        let resp = test::call_service(&service, req).await;

        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "ok");
    }

    #[actix_rt::test]
    async fn test_isolated_apps() {
        let test_app1 = TestApp::new().await;
        let test_app2 = TestApp::new().await;

        // Test IDs should be different
        assert_ne!(test_app1.config.test_id, test_app2.config.test_id);

        // Database paths should be different
        assert_ne!(test_app1.database.path(), test_app2.database.path());

        // Both should have empty project lists initially
        let projects1 = test_app1.db().get_all_projects().unwrap();
        let projects2 = test_app2.db().get_all_projects().unwrap();

        assert_eq!(projects1.len(), 0);
        assert_eq!(projects2.len(), 0);
    }

    #[actix_rt::test]
    async fn test_app_isolation() {
        let test_app1 = TestApp::new().await;
        let test_app2 = TestApp::new().await;

        // Create a project in app1
        let project = nocodo_manager::models::Project {
            id: 200, // Test ID
            name: "Isolation Test".to_string(),
            path: "/tmp/isolation-test".to_string(),
            description: None,
            parent_id: None,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        test_app1.db().create_project(&project).unwrap();

        // App1 should have the project
        let projects1 = test_app1.db().get_all_projects().unwrap();
        assert_eq!(projects1.len(), 1);
        assert_eq!(projects1[0].name, "Isolation Test");

        // App2 should still have empty project list
        let projects2 = test_app2.db().get_all_projects().unwrap();
        assert_eq!(projects2.len(), 0);
    }
}
