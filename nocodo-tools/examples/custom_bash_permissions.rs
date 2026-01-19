//! Example showing how to create tool executors with custom bash permissions

use nocodo_tools::{
    bash::{BashExecutor, BashPermissions},
    types::{BashRequest, ToolRequest},
    ToolExecutor,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Example 1: Tesseract-only executor
    println!("Example 1: Tesseract-only permissions");
    let tesseract_perms = BashPermissions::minimal(vec!["tesseract"]);
    let tesseract_executor = create_executor_with_permissions(tesseract_perms);

    // This will succeed (permission check passes, actual execution may fail if tesseract not installed)
    test_command(&tesseract_executor, "tesseract --help").await;

    // This will fail (not allowed)
    test_command(&tesseract_executor, "ls -la").await;

    println!();

    // Example 2: Read-only executor
    println!("Example 2: Read-only permissions");
    let readonly_perms = BashPermissions::read_only();
    let readonly_executor = create_executor_with_permissions(readonly_perms);

    // These will succeed
    test_command(&readonly_executor, "ls -la").await;
    test_command(&readonly_executor, "cat /etc/hosts").await;

    // This will fail (write operation)
    test_command(&readonly_executor, "echo test > /tmp/test.txt").await;

    println!();

    // Example 3: Multiple specific commands
    println!("Example 3: Multiple specific commands");
    let multi_perms = BashPermissions::only_allow(vec!["tesseract*", "convert*"]);
    let multi_executor = create_executor_with_permissions(multi_perms);

    test_command(&multi_executor, "tesseract --help").await;
    test_command(&multi_executor, "convert --help").await;
    test_command(&multi_executor, "ls -la").await; // Will fail

    println!();

    // Example 4: Default permissions (backward compatibility)
    println!("Example 4: Default permissions");
    let default_executor = ToolExecutor::new(PathBuf::from("."));

    // These should all succeed with default permissions
    test_command(&default_executor, "echo hello").await;
    test_command(&default_executor, "pwd").await;

    println!();

    // Example 5: Disabled bash tool
    println!("Example 5: Disabled bash tool");
    let no_bash_executor = ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(None)
        .build();

    // This will fail because there's no bash executor
    test_command(&no_bash_executor, "echo hello").await;

    Ok(())
}

fn create_executor_with_permissions(perms: BashPermissions) -> ToolExecutor {
    let bash_executor = BashExecutor::new(perms, 120).expect("Failed to create bash executor");

    ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(Some(Box::new(bash_executor)))
        .build()
}

async fn test_command(executor: &ToolExecutor, command: &str) {
    let request = ToolRequest::Bash(BashRequest {
        command: command.to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    match executor.execute(request).await {
        Ok(_) => println!("✓ Allowed: {}", command),
        Err(e) => {
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("not allowed") || error_msg.contains("denied") {
                println!("✗ Denied: {} - {}", command, e);
            } else {
                println!("✓ Allowed (but failed): {} - {}", command, e);
            }
        }
    }
}
