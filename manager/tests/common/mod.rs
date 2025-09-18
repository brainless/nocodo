//! Common test utilities and infrastructure for isolated testing
//!
//! This module provides the foundation for running isolated API E2E tests
//! with complete separation between test environments.

pub mod app;
pub mod config;
pub mod database;
pub mod fixtures;
pub mod logging;

pub use app::TestApp;
pub use config::TestConfig;
pub use database::TestDatabase;
pub use fixtures::TestDataGenerator;
pub use logging::{TestLogger, TestLoggerGuard, init_test_logging};

#[cfg(test)]
mod isolation_tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_complete_isolation() {
        // Create two completely isolated test environments
        let test_app1 = TestApp::new();
        let test_app2 = TestApp::new();

        // Verify different test IDs
        assert_ne!(test_app1.test_config().test_id, test_app2.test_config().test_id);

        // Verify different database paths
        assert_ne!(test_app1.database.path(), test_app2.database.path());

        // Verify different socket paths
        assert_ne!(test_app1.test_config().socket_path(), test_app2.test_config().socket_path());

        // Verify different log paths
        assert_ne!(test_app1.test_config().log_path(), test_app2.test_config().log_path());

        // Verify different projects directories
        assert_ne!(test_app1.test_config().projects_dir(), test_app2.test_config().projects_dir());

        // Both should start with empty databases
        let projects1 = test_app1.db().get_all_projects().unwrap();
        let projects2 = test_app2.db().get_all_projects().unwrap();
        assert_eq!(projects1.len(), 0);
        assert_eq!(projects2.len(), 0);
    }

    #[test]
    fn test_parallel_isolation() {
        let handles: Vec<_> = (0..3)
            .map(|i| {
                thread::spawn(move || {
                    let test_app = TestApp::new();

                    // Create a unique project in each thread
                    let project = TestDataGenerator::create_project_custom(
                        &format!("parallel-project-{}", i),
                        &format!("/tmp/parallel-project-{}", i),
                        Some("rust"),
                        Some("actix-web"),
                        Some("initialized"),
                    );

                    test_app.db().create_project(&project).unwrap();

                    // Verify the project exists in this thread's database
                    let projects = test_app.db().get_all_projects().unwrap();
                    assert_eq!(projects.len(), 1);
                    assert_eq!(projects[0].name, format!("parallel-project-{}", i));

                    // Return the test ID and project count for verification
                    (test_app.test_config().test_id.clone(), projects.len())
                })
            })
            .collect();

        // Collect results from all threads
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All should have different test IDs and same project count
        let test_ids: std::collections::HashSet<_> = results.iter().map(|(id, _)| id).collect();
        assert_eq!(test_ids.len(), 3); // All IDs should be unique

        for (_, count) in &results {
            assert_eq!(*count, 1); // Each thread should have exactly 1 project
        }
    }

    #[test]
    fn test_resource_cleanup() {
        let temp_paths: Vec<std::path::PathBuf> = (0..2)
            .map(|_| {
                let test_app = TestApp::new();
                let db_path = test_app.database.path().clone();
                let log_path = test_app.test_config().log_path();

                // Verify files exist while in scope
                assert!(db_path.exists());
                assert!(log_path.exists());

                db_path
            })
            .collect();

        // After going out of scope, files should be cleaned up
        thread::sleep(Duration::from_millis(100)); // Give cleanup time

        for path in temp_paths {
            // The database file should be gone (temp directory cleanup)
            assert!(!path.exists() || !std::fs::read(&path).is_ok());
        }
    }

    #[test]
    fn test_database_transaction_isolation() {
        let test_app = TestApp::new();

        // Start a transaction in one "session"
        let db = test_app.db();
        let tx1 = db.connection.lock().unwrap();
        tx1.execute("BEGIN", []).unwrap();

        // Create a project in the transaction
        let project1 = TestDataGenerator::create_project(Some("tx-project-1"), Some("/tmp/tx-1"));
        db.create_project(&project1).unwrap();

        // In the same transaction, should see the project
        let projects_in_tx = db.get_all_projects().unwrap();
        assert_eq!(projects_in_tx.len(), 1);

        // Start another "session" (simulated by getting another connection)
        let tx2 = db.connection.lock().unwrap();
        tx2.execute("BEGIN", []).unwrap();

        // Second transaction should not see uncommitted changes from first
        let projects_in_tx2 = db.get_all_projects().unwrap();
        assert_eq!(projects_in_tx2.len(), 0); // Should be empty due to transaction isolation

        // Commit first transaction
        tx1.execute("COMMIT", []).unwrap();

        // Now second transaction should see the committed changes
        let projects_after_commit = db.get_all_projects().unwrap();
        assert_eq!(projects_after_commit.len(), 1);
    }

    #[test]
    fn test_logging_isolation() {
        let logger1 = TestLogger::new();
        let logger2 = TestLogger::new();

        // Log different messages to each logger
        tracing::info!("Message for logger 1: {}", logger1.config().test_id);
        thread::sleep(Duration::from_millis(10));

        tracing::info!("Message for logger 2: {}", logger2.config().test_id);
        thread::sleep(Duration::from_millis(10));

        // Each logger should only contain its own messages
        let logs1 = logger1.read_logs().unwrap();
        let logs2 = logger2.read_logs().unwrap();

        assert!(logs1.contains(&logger1.config().test_id));
        assert!(logs2.contains(&logger2.config().test_id));

        // Logger 1 should not contain logger 2's message
        assert!(!logs1.contains(&logger2.config().test_id));
        assert!(!logs2.contains(&logger1.config().test_id));
    }

    #[test]
    fn test_fixture_consistency() {
        let generator = TestDataGenerator::create_complete_scenario();
        let (project, work, messages) = generator;

        // Verify the complete scenario structure
        assert_eq!(project.name, "scenario-project");
        assert_eq!(work.title, "Scenario Work Session");
        assert_eq!(messages.len(), 3);

        // Verify message sequence and authors
        assert_eq!(messages[0].sequence_order, 0);
        assert_eq!(messages[1].sequence_order, 1);
        assert_eq!(messages[2].sequence_order, 2);

        assert!(matches!(messages[0].author_type, nocodo_manager::models::MessageAuthorType::User));
        assert!(matches!(messages[1].author_type, nocodo_manager::models::MessageAuthorType::Ai));
        assert!(matches!(messages[2].author_type, nocodo_manager::models::MessageAuthorType::User));

        // Verify relationships
        assert_eq!(work.project_id, Some(project.id));
        for message in &messages {
            assert_eq!(message.work_id, work.id);
        }
    }

    #[test]
    fn test_performance_baseline() {
        let test_app = TestApp::new();

        // Measure time to create multiple projects
        let start = std::time::Instant::now();

        for i in 0..10 {
            let project = TestDataGenerator::create_project_custom(
                &format!("perf-project-{}", i),
                &format!("/tmp/perf-project-{}", i),
                Some("rust"),
                Some("actix-web"),
                Some("initialized"),
            );
            test_app.db().create_project(&project).unwrap();
        }

        let duration = start.elapsed();

        // Should complete in reasonable time (less than 1 second for 10 operations)
        assert!(duration < Duration::from_secs(1));

        // Verify all projects were created
        let projects = test_app.db().get_all_projects().unwrap();
        assert_eq!(projects.len(), 10);
    }
}