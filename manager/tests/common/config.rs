use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

use nocodo_manager::config::{
    ApiKeysConfig, AppConfig, DatabaseConfig, ServerConfig, SocketConfig,
};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Get a unique test identifier for isolation
pub fn get_unique_test_id() -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    format!("test-{}-{}", pid, counter)
}

/// Test-specific configuration that provides complete isolation
#[derive(Debug)]
pub struct TestConfig {
    pub temp_dir: TempDir,
    pub config: AppConfig,
    pub test_id: String,
}

impl TestConfig {
    /// Create a new isolated test configuration
    pub fn new() -> Self {
        let test_id = get_unique_test_id();
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory for test");

        // Create isolated database path
        let db_path = temp_dir.path().join(format!("nocodo-test-{}.db", test_id));

        // Create isolated socket path
        let socket_path = temp_dir
            .path()
            .join(format!("nocodo-test-{}.sock", test_id));

        // Create isolated log path
        let _log_path = temp_dir.path().join(format!("nocodo-test-{}.log", test_id));

        let config = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0, // Use port 0 for automatic assignment
            },
            database: DatabaseConfig { path: db_path },
            socket: SocketConfig {
                path: socket_path.to_string_lossy().to_string(),
            },
            api_keys: Some(ApiKeysConfig {
                xai_api_key: Some("test-xai-key".to_string()),
                openai_api_key: Some("test-openai-key".to_string()),
                anthropic_api_key: Some("test-anthropic-key".to_string()),
            }),
            projects: None,
            jwt_secret: Some("test-jwt-secret-key-for-testing-purposes-only".to_string()),
        };

        Self {
            temp_dir,
            config,
            test_id,
        }
    }

    /// Get the isolated database path
    pub fn db_path(&self) -> &PathBuf {
        &self.config.database.path
    }

    /// Get the isolated socket path
    pub fn socket_path(&self) -> &str {
        &self.config.socket.path
    }

    /// Get the isolated log path
    pub fn log_path(&self) -> PathBuf {
        self.temp_dir
            .path()
            .join(format!("nocodo-test-{}.log", self.test_id))
    }

    /// Get the isolated projects directory
    pub fn projects_dir(&self) -> PathBuf {
        self.temp_dir.path().join("projects")
    }

    /// Get the temp directory path
    #[allow(dead_code)]
    pub fn temp_dir_path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Create the projects directory if it doesn't exist
    pub fn ensure_projects_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.projects_dir())
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_test_ids() {
        let config1 = TestConfig::new();
        let config2 = TestConfig::new();

        assert_ne!(config1.test_id, config2.test_id);
        assert!(config1.test_id.starts_with("test-"));
        assert!(config2.test_id.starts_with("test-"));
    }

    #[test]
    fn test_isolated_paths() {
        let config = TestConfig::new();

        // Database path should be within temp directory
        assert!(config.db_path().starts_with(config.temp_dir.path()));
        assert!(config.db_path().to_string_lossy().contains(&config.test_id));

        // Socket path should be within temp directory
        assert!(config.socket_path().contains(&config.test_id));

        // Log path should be within temp directory
        assert!(config.log_path().starts_with(config.temp_dir.path()));
        assert!(config
            .log_path()
            .to_string_lossy()
            .contains(&config.test_id));

        // Projects directory should be within temp directory
        assert!(config.projects_dir().starts_with(config.temp_dir.path()));
    }

    #[test]
    fn test_projects_dir_creation() {
        let config = TestConfig::new();

        // Directory shouldn't exist initially
        assert!(!config.projects_dir().exists());

        // Create it
        config.ensure_projects_dir().unwrap();

        // Should exist now
        assert!(config.projects_dir().exists());
        assert!(config.projects_dir().is_dir());
    }
}
