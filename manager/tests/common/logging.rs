use std::path::PathBuf;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

use super::config::TestConfig;

/// TestLogger provides isolated logging for tests
pub struct TestLogger {
    config: TestConfig,
    _guard: tracing::subscriber::DefaultGuard,
}

impl TestLogger {
    /// Initialize isolated logging for tests
    pub fn new() -> Self {
        let config = TestConfig::new();

        // Create a unique log file for this test
        let log_file = std::fs::File::create(config.log_path())
            .expect("Failed to create test log file");

        // Set up tracing subscriber with file output
        let file_writer = fmt::writer::BoxMakeWriter::new(log_file);

        let subscriber = tracing_subscriber::registry()
            .with(
                EnvFilter::from_default_env()
                    .add_directive("nocodo_manager=debug".parse().unwrap())
                    .add_directive("actix_web=info".parse().unwrap())
                    .add_directive("rusqlite=warn".parse().unwrap()),
            )
            .with(
                fmt::layer()
                    .with_writer(file_writer)
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(false)
                    .with_line_number(false)
                    .compact(),
            );

        let guard = tracing::subscriber::set_default(subscriber);

        Self {
            config,
            _guard: guard,
        }
    }

    /// Get the log file path
    pub fn log_path(&self) -> PathBuf {
        self.config.log_path()
    }

    /// Get the test configuration
    pub fn config(&self) -> &TestConfig {
        &self.config
    }

    /// Read the current log contents
    pub fn read_logs(&self) -> std::io::Result<String> {
        std::fs::read_to_string(self.log_path())
    }

    /// Check if logs contain a specific pattern
    pub fn contains(&self, pattern: &str) -> std::io::Result<bool> {
        let logs = self.read_logs()?;
        Ok(logs.contains(pattern))
    }

    /// Clear the log file
    pub fn clear_logs(&self) -> std::io::Result<()> {
        std::fs::write(self.log_path(), "")
    }

    /// Get log lines as a vector
    pub fn log_lines(&self) -> std::io::Result<Vec<String>> {
        let content = self.read_logs()?;
        Ok(content.lines().map(|s| s.to_string()).collect())
    }
}

impl Default for TestLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize global test logging (use once per test binary)
pub fn init_test_logging() {
    // Only initialize if not already initialized
    if tracing::dispatcher::has_been_set() {
        return;
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("nocodo_manager=debug".parse().unwrap())
                .add_directive("actix_web=info".parse().unwrap())
                .add_directive("rusqlite=warn".parse().unwrap()),
        )
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .init();
}

/// TestLoggerGuard ensures proper cleanup of test logging
pub struct TestLoggerGuard {
    pub logger: TestLogger,
}

impl TestLoggerGuard {
    pub fn new() -> Self {
        Self {
            logger: TestLogger::new(),
        }
    }
}

impl Default for TestLoggerGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_isolated_logging() {
        let logger1 = TestLogger::new();
        let logger2 = TestLogger::new();

        // Log files should be different
        assert_ne!(logger1.log_path(), logger2.log_path());

        // Both log files should exist
        assert!(logger1.log_path().exists());
        assert!(logger2.log_path().exists());
    }

    #[test]
    fn test_log_writing() {
        let logger = TestLogger::new();

        // Initially logs should be empty or minimal
        let initial_logs = logger.read_logs().unwrap();
        assert!(initial_logs.is_empty() || initial_logs.lines().count() <= 2);

        // Log some test messages
        tracing::info!("Test log message 1");
        tracing::debug!("Test debug message");
        tracing::warn!("Test warning message");

        // Give logging some time to write
        thread::sleep(Duration::from_millis(10));

        // Check that logs contain our messages
        let logs = logger.read_logs().unwrap();
        assert!(logs.contains("Test log message 1"));
        assert!(logs.contains("Test debug message"));
        assert!(logs.contains("Test warning message"));
    }

    #[test]
    fn test_log_contains() {
        let logger = TestLogger::new();

        tracing::info!("Unique test message: {}", logger.config().test_id);

        thread::sleep(Duration::from_millis(10));

        assert!(logger.contains("Unique test message").unwrap());
        assert!(logger.contains(&logger.config().test_id).unwrap());
        assert!(!logger.contains("Non-existent message").unwrap());
    }

    #[test]
    fn test_log_lines() {
        let logger = TestLogger::new();

        tracing::info!("Line 1");
        tracing::info!("Line 2");
        tracing::info!("Line 3");

        thread::sleep(Duration::from_millis(10));

        let lines = logger.log_lines().unwrap();
        let info_lines: Vec<_> = lines
            .iter()
            .filter(|line| line.contains("Line "))
            .collect();

        assert_eq!(info_lines.len(), 3);
        assert!(info_lines.iter().any(|line| line.contains("Line 1")));
        assert!(info_lines.iter().any(|line| line.contains("Line 2")));
        assert!(info_lines.iter().any(|line| line.contains("Line 3")));
    }

    #[test]
    fn test_log_clearing() {
        let logger = TestLogger::new();

        tracing::info!("Message before clear");

        thread::sleep(Duration::from_millis(10));

        assert!(logger.contains("Message before clear").unwrap());

        logger.clear_logs().unwrap();

        let logs_after_clear = logger.read_logs().unwrap();
        assert!(!logs_after_clear.contains("Message before clear"));

        tracing::info!("Message after clear");

        thread::sleep(Duration::from_millis(10));

        let final_logs = logger.read_logs().unwrap();
        assert!(final_logs.contains("Message after clear"));
        assert!(!final_logs.contains("Message before clear"));
    }

    #[test]
    fn test_logger_guard() {
        let guard = TestLoggerGuard::new();

        tracing::info!("Test message with guard");

        thread::sleep(Duration::from_millis(10));

        assert!(guard.logger.contains("Test message with guard").unwrap());
    }
}