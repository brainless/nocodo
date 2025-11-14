use nocodo_manager::bash_executor::{BashExecutor, BashExecutorError};
use nocodo_manager::bash_permissions::{BashPermissions, PermissionRule};
use nocodo_manager::models::{BashRequest, BashResponse};
use nocodo_manager::tools::ToolExecutor;
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing Bash Tool Integration ===");
    
    // Initialize the tool executor
    let project_path = std::env::current_dir()?;
    let tool_executor = ToolExecutor::new(project_path.to_string_lossy().to_string());
    
    // Test commands
    let test_commands = vec![
        ("echo 'Hello from bash tool!'", "Basic echo test"),
        ("git status", "Git status test"),
        ("ls -la", "Directory listing test"),
        ("cargo check", "Cargo check test"),
        ("pwd", "Print working directory test"),
        ("whoami", "Current user test"),
        ("date", "Date test"),
        ("uname -r", "Kernel version test"),
        ("ps aux | head -3", "Process list test"),
        ("echo 'test content' | grep test", "Pipe test"),
    ];
    
    for (command, description) in test_commands {
        println!("\n--- {} ---", description);
        println!("Command: {}", command);
        
        let request = BashRequest {
            command: command.to_string(),
            timeout_secs: Some(10),
            working_directory: None,
        };
        
        match tool_executor.execute_bash(request).await {
            Ok(response) => {
                println!("Exit code: {}", response.exit_code);
                if !response.stdout.is_empty() {
                    println!("STDOUT:\n{}", response.stdout);
                }
                if !response.stderr.is_empty() {
                    println!("STDERR:\n{}", response.stderr);
                }
                println!("✅ Success");
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
    
    // Test permission denied
    println!("\n--- Testing Permission Denied ---");
    let dangerous_request = BashRequest {
        command: "rm -rf /".to_string(),
        timeout_secs: Some(5),
        working_directory: None,
    };
    
    match tool_executor.execute_bash(dangerous_request).await {
        Ok(_) => {
            println!("❌ Dangerous command was allowed (this should not happen)");
        }
        Err(e) => {
            println!("✅ Dangerous command correctly blocked: {}", e);
        }
    }
    
    println!("\n=== All tests completed ===");
    Ok(())
}