use std::path::PathBuf;
use std::sync::Arc;

use nocodo_manager::database::Database;
use nocodo_manager::error::AppResult;

use super::config::TestConfig;

/// TestDatabase provides isolated database management for tests
pub struct TestDatabase {
    pub database: Arc<Database>,
    pub config: TestConfig,
}

impl TestDatabase {
    /// Create a new isolated test database
    pub fn new() -> AppResult<Self> {
        let config = TestConfig::new();
        let database = Arc::new(Database::new(config.db_path())?);

        Ok(Self { database, config })
    }

    /// Get the database path
    pub fn path(&self) -> &PathBuf {
        self.config.db_path()
    }

    /// Get the test configuration
    pub fn config(&self) -> &TestConfig {
        &self.config
    }

    /// Clean up the test database (called automatically on drop)
    pub fn cleanup(&self) -> std::io::Result<()> {
        // The temp directory will be cleaned up automatically when TestConfig is dropped
        // But we can add additional cleanup logic here if needed
        Ok(())
    }

    /// Get a reference to the underlying database
    pub fn db(&self) -> &Arc<Database> {
        &self.database
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        // Cleanup is handled by the TestConfig's TempDir
        tracing::debug!("TestDatabase cleanup: {}", self.config.test_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_isolated_database_creation() {
        let test_db = TestDatabase::new().unwrap();

        // Database file should exist
        assert!(test_db.path().exists());
        assert!(test_db.path().is_file());

        // Database should be accessible
        let projects = test_db.db().get_all_projects().unwrap();
        assert_eq!(projects.len(), 0); // Should be empty initially
    }

    #[test]
    fn test_database_isolation() {
        let test_db1 = TestDatabase::new().unwrap();
        let test_db2 = TestDatabase::new().unwrap();

        // Database paths should be different
        assert_ne!(test_db1.path(), test_db2.path());

        // Both should exist
        assert!(test_db1.path().exists());
        assert!(test_db2.path().exists());

        // Test IDs should be different
        assert_ne!(test_db1.config.test_id, test_db2.config.test_id);
    }

    #[test]
    fn test_database_operations() {
        let test_db = TestDatabase::new().unwrap();

        // Create a test project
        let project = nocodo_manager::models::Project {
            id: "test-project-id".to_string(),
            name: "Test Project".to_string(),
            path: "/tmp/test-path".to_string(),
            language: Some("rust".to_string()),
            framework: Some("actix-web".to_string()),
            status: "initialized".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            technologies: None,
        };

        // Insert project
        test_db.db().create_project(&project).unwrap();

        // Retrieve project
        let retrieved = test_db.db().get_project_by_id("test-project-id").unwrap();
        assert_eq!(retrieved.name, "Test Project");
        assert_eq!(retrieved.language, Some("rust".to_string()));

        // List all projects
        let projects = test_db.db().get_all_projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Test Project");
    }

    #[test]
    fn test_database_cleanup() {
        let temp_path: PathBuf;
        {
            let test_db = TestDatabase::new().unwrap();
            temp_path = test_db.path().clone();

            // Database should exist while in scope
            assert!(temp_path.exists());

            // Create some data
            let project = nocodo_manager::models::Project {
                id: "cleanup-test".to_string(),
                name: "Cleanup Test".to_string(),
                path: "/tmp/cleanup".to_string(),
                language: Some("rust".to_string()),
                framework: None,
                status: "initialized".to_string(),
                created_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
                technologies: None,
            };
            test_db.db().create_project(&project).unwrap();
        }
        // After drop, the temp directory should be cleaned up
        // Note: This test may be flaky in some environments, but it's good to verify cleanup
        if temp_path.exists() {
            // If it still exists, at least verify the database file is gone
            assert!(!temp_path.exists() || fs::read(&temp_path).is_err());
        }
    }
}