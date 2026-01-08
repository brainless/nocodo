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

/// TestApp provides a fully configured test application with isolated resources (LLM agent removed)
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
