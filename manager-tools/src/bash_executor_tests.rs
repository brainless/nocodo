#[cfg(test)]
mod tests {
    use crate::bash_executor::BashExecutor;
    use crate::bash_permissions::{BashPermissions, PermissionRule};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_bash_executor_creation() {
        let executor = BashExecutor::new(BashPermissions::default(), 30).unwrap();
        
        // Test that executor was created successfully
        assert!(executor.get_permissions().is_command_allowed("echo test"));
    }

    #[tokio::test]
    async fn test_bash_executor_simple_command() {
        let executor = BashExecutor::new(BashPermissions::default(), 10).unwrap();
        
        let result = executor.execute("echo 'Hello, World!'", Some(5)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.exit_code, 0);
        assert!(execution_result.stdout.contains("Hello, World!"));
        assert!(execution_result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_bash_executor_command_with_stderr() {
        let executor = BashExecutor::new(BashPermissions::default(), 10).unwrap();
        
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
        let executor = BashExecutor::new(BashPermissions::default(), 10).unwrap();
        
        let result = executor.execute("nonexistent_command_12345", Some(5)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert!(execution_result.exit_code != 0);
        assert!(execution_result.stderr.contains("not found") || execution_result.stderr.contains("command not found"));
    }

    #[tokio::test]
    async fn test_bash_executor_timeout() {
        let permissions = BashPermissions::new(vec![PermissionRule::allow("sleep*").unwrap()]);
        let executor = BashExecutor::new(permissions, 10).unwrap();
        
        // This command should run longer than the timeout
        let result = executor.execute("sleep 10", Some(2)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert!(execution_result.timed_out);
        assert_eq!(execution_result.exit_code, 124);
    }

    #[tokio::test]
    async fn test_bash_executor_permission_denied() {
        let executor = BashExecutor::new(BashPermissions::default(), 10).unwrap();
        
        // This command should be blocked by permissions
        let result = executor.execute("rm -rf /", Some(5)).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("denied"));
    }

    #[tokio::test]
    async fn test_bash_executor_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        
        let permissions = BashPermissions::new(vec![
            PermissionRule::allow("ls*").unwrap(),
            PermissionRule::allow("test*").unwrap(),
        ]);
        let executor = BashExecutor::new(permissions, 10).unwrap();
        
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
        
        let permissions = BashPermissions::new(vec![PermissionRule::allow("git*").unwrap()]);
        let executor = BashExecutor::new(permissions, 10).unwrap();
        
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
        let permissions = BashPermissions::new(vec![
            PermissionRule::allow("cargo*").unwrap(),
            PermissionRule::allow("ls*").unwrap(),
        ]);
        let executor = BashExecutor::new(permissions, 10).unwrap();
        
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
        let mut executor = BashExecutor::new(BashPermissions::default(), 10).unwrap();
        
        // Get initial permissions
        let initial_perms = executor.get_permissions();
        assert!(initial_perms.is_command_allowed("git status"));
        
        // Update permissions to block git
        let new_perms = BashPermissions::new(vec![PermissionRule::deny("git*").unwrap()]);
        executor.update_permissions(new_perms);
        
        // Check that git is now blocked
        let updated_perms = executor.get_permissions();
        assert!(!updated_perms.is_command_allowed("git status"));
    }

    #[tokio::test]
    async fn test_bash_executor_default_timeout() {
        let permissions = BashPermissions::new(vec![PermissionRule::allow("sleep*").unwrap()]);
        let executor = BashExecutor::new(permissions, 5).unwrap(); // 5 second default timeout
        
        // Test with default timeout (should use the 5 second default)
        let result = executor.execute("sleep 10", None).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert!(execution_result.timed_out);
        assert_eq!(execution_result.exit_code, 124);
    }

    #[tokio::test]
    async fn test_bash_executor_complex_command() {
        let temp_dir = TempDir::new().unwrap();
        
        let permissions = BashPermissions::new(vec![
            PermissionRule::allow("cat*").unwrap(),
            PermissionRule::allow("grep*").unwrap(),
            PermissionRule::allow("wc*").unwrap(),
        ]);
        let executor = BashExecutor::new(permissions, 10).unwrap();
        
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
        let executor = BashExecutor::new(BashPermissions::default(), 10).unwrap();
        
        // Test environment variable usage
        let result = executor.execute("echo $HOME", Some(5)).await;
        
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.exit_code, 0);
        assert!(!execution_result.stdout.trim().is_empty());
    }
}