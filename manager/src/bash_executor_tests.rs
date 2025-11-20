#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;
    use tokio_test;

    #[tokio::test]
    async fn test_bash_executor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path.clone(),
            BashPermissions::default(),
            30,
        );
        
        assert_eq!(executor.project_path, project_path);
        assert_eq!(executor.default_timeout_secs, 30);
    }

    #[tokio::test]
    async fn test_bash_executor_simple_command() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        let result = executor.execute("echo 'Hello, World!'", Some(5)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.exit_code, 0);
        assert!(execution_result.stdout.contains("Hello, World!"));
        assert!(execution_result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_bash_executor_command_with_stderr() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        // This command writes to stderr
        let result = executor.execute("echo 'Error message' >&2", Some(5)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.exit_code, 0);
        assert!(execution_result.stdout.is_empty());
        assert!(execution_result.stderr.contains("Error message"));
    }

    #[tokio::test]
    async fn test_bash_executor_nonexistent_command() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        let result = executor.execute("nonexistent_command_12345", Some(5)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert!(execution_result.exit_code != 0);
        assert!(execution_result.stderr.contains("not found") || execution_result.stderr.contains("command not found"));
    }

    #[tokio::test]
    async fn test_bash_executor_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        // This command should run longer than the timeout
        let result = executor.execute("sleep 10", Some(2)).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BashExecutorError::Timeout(_)));
    }

    #[tokio::test]
    async fn test_bash_executor_permission_denied() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        // This command should be blocked by permissions
        let result = executor.execute("rm -rf /", Some(5)).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BashExecutorError::PermissionDenied(_)));
    }

    #[tokio::test]
    async fn test_bash_executor_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path.clone(),
            BashPermissions::default(),
            10,
        );
        
        // Create a test file in the project directory
        let test_file = temp_dir.path().join("test_file.txt");
        std::fs::write(&test_file, "test content").unwrap();
        
        // List files in the project directory
        let result = executor.execute("ls test_file.txt", Some(5)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.exit_code, 0);
        assert!(execution_result.stdout.contains("test_file.txt"));
    }

    #[tokio::test]
    async fn test_bash_executor_git_commands() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        // Initialize git repository
        let init_result = executor.execute("git init", Some(5)).await;
        assert!(init_result.is_ok());
        assert_eq!(init_result.unwrap().exit_code, 0);
        
        // Check git status
        let status_result = executor.execute("git status", Some(5)).await;
        assert!(status_result.is_ok());
        assert_eq!(status_result.unwrap().exit_code, 0);
        
        // Create a test file and add it
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test").unwrap();
        
        let add_result = executor.execute("git add test.txt", Some(5)).await;
        assert!(add_result.is_ok());
        assert_eq!(add_result.unwrap().exit_code, 0);
    }

    #[tokio::test]
    async fn test_bash_executor_cargo_commands() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        // Initialize a minimal Rust project
        let init_result = executor.execute("cargo init --bin test_project", Some(10)).await;
        assert!(init_result.is_ok());
        assert_eq!(init_result.unwrap().exit_code, 0);
        
        // Check if project was created
        let check_result = executor.execute("ls test_project", Some(5)).await;
        assert!(check_result.is_ok());
        let check_output = check_result.unwrap();
        assert_eq!(check_output.exit_code, 0);
        assert!(check_output.stdout.contains("Cargo.toml"));
    }

    #[tokio::test]
    async fn test_bash_executor_permissions_get_set() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let mut executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        // Get initial permissions
        let initial_perms = executor.get_permissions();
        assert!(initial_perms.is_command_allowed("git status"));
        
        // Update permissions to block git
        let mut new_perms = BashPermissions::default();
        let deny_git_rule = PermissionRule {
            pattern: glob::Pattern::new("git*").unwrap(),
            allow: false,
            description: "Block git commands".to_string(),
        };
        new_perms.add_rule(deny_git_rule);
        
        executor.update_permissions(new_perms);
        
        // Check that git is now blocked
        let updated_perms = executor.get_permissions();
        assert!(!updated_perms.is_command_allowed("git status"));
    }

    #[tokio::test]
    async fn test_bash_executor_default_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            5, // 5 second default timeout
        );
        
        // Test with default timeout (should use the 5 second default)
        let result = executor.execute("sleep 10", None).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BashExecutorError::Timeout(_)));
    }

    #[tokio::test]
    async fn test_bash_executor_complex_command() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "hello\nworld\ntest").unwrap();
        
        // Run a complex command with pipes and redirects
        let result = executor.execute("cat test.txt | grep hello | wc -l", Some(5)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.exit_code, 0);
        assert!(execution_result.stdout.trim().parse::<u32>().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_bash_executor_environment_variables() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();
        
        let executor = BashExecutor::new(
            project_path,
            BashPermissions::default(),
            10,
        );
        
        // Test environment variable usage
        let result = executor.execute("echo $HOME", Some(5)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.exit_code, 0);
        assert!(!execution_result.stdout.trim().is_empty());
    }
}