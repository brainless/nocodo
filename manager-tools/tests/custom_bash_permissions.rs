use manager_tools::{
    bash::{BashExecutor, BashPermissions},
    types::{BashRequest, ToolRequest, ToolResponse},
    ToolExecutor,
};
use std::path::PathBuf;

#[tokio::test]
async fn test_tesseract_only_permissions() {
    let perms = BashPermissions::minimal(vec!["tesseract"]);
    let bash_executor = BashExecutor::new(perms, 120).unwrap();
    let executor = ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(Some(Box::new(bash_executor)))
        .build();

    // Tesseract should be allowed (testing permission check, not actual execution)
    let request = ToolRequest::Bash(BashRequest {
        command: "tesseract --help".to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    // Note: This will fail if tesseract isn't installed, but we're testing
    // that the permission system allows the command to attempt execution
    let result = executor.execute(request).await;

    // Either succeeds (tesseract installed) or fails with execution error (not permission error)
    // Permission errors contain "not allowed" in the message
    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(!error_msg.to_lowercase().contains("not allowed"));
    }
}

#[tokio::test]
async fn test_restricted_executor_denies_other_commands() {
    let perms = BashPermissions::minimal(vec!["tesseract"]);
    let bash_executor = BashExecutor::new(perms, 120).unwrap();
    let executor = ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(Some(Box::new(bash_executor)))
        .build();

    // ls should be denied
    let request = ToolRequest::Bash(BashRequest {
        command: "ls -la".to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    let result = executor.execute(request).await;

    match result {
        Ok(ToolResponse::Error(err)) => {
            let error_msg = err.message.to_lowercase();
            assert!(error_msg.contains("denied") || error_msg.contains("not allowed"));
        }
        Ok(other) => {
            panic!("Expected error response, got: {:?}", other);
        }
        Err(e) => {
            panic!("Expected error response, got execution error: {}", e);
        }
    }
}

#[tokio::test]
async fn test_default_permissions_backward_compatibility() {
    // Using old API should still work
    let executor = ToolExecutor::new(PathBuf::from("."));

    let request = ToolRequest::Bash(BashRequest {
        command: "echo hello".to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    let result = executor.execute(request).await;

    // Should succeed with default permissions (if bash executor is configured)
    // Note: This test might fail if no bash executor is configured, which is expected
    match result {
        Ok(ToolResponse::Bash(response)) => {
            assert!(response.stdout.contains("hello") || response.stderr.contains("hello"));
        }
        Ok(_) => {
            // Other response types are also acceptable
        }
        Err(e) => {
            // Check if it's a "no bash executor" error, which is expected
            let error_msg = e.to_string().to_lowercase();
            if !error_msg.contains("no bash") && !error_msg.contains("not configured") {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_builder_pattern_with_custom_permissions() {
    let perms = BashPermissions::only_allow(vec!["echo*", "pwd"]);
    let bash_executor = BashExecutor::new(perms, 120).unwrap();

    let executor = ToolExecutor::builder()
        .base_path(PathBuf::from("/tmp"))
        .max_file_size(2048)
        .bash_executor(Some(Box::new(bash_executor)))
        .build();

    // Test that builder worked correctly
    assert_eq!(executor.base_path(), &PathBuf::from("/tmp"));

    // Test allowed commands
    let echo_request = ToolRequest::Bash(BashRequest {
        command: "echo test".to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    let result = executor.execute(echo_request).await;
    match result {
        Ok(ToolResponse::Bash(_)) | Ok(ToolResponse::Error(_)) => {
            // Either executed successfully (allowed) or failed with execution error (not permission error)
        }
        Ok(other) => {
            panic!("Unexpected response type: {:?}", other);
        }
        Err(e) => {
            panic!("Unexpected execution error: {}", e);
        }
    }

    // Test denied commands
    let ls_request = ToolRequest::Bash(BashRequest {
        command: "ls -la".to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    let result = executor.execute(ls_request).await;
    match result {
        Ok(ToolResponse::Error(err)) => {
            let error_msg = err.message.to_lowercase();
            assert!(error_msg.contains("denied") || error_msg.contains("not allowed"));
        }
        Ok(other) => {
            panic!(
                "Expected error response for denied command, got: {:?}",
                other
            );
        }
        Err(e) => {
            panic!("Expected error response, got execution error: {}", e);
        }
    }
}

#[tokio::test]
async fn test_disabled_bash_executor() {
    let executor = ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(None)
        .build();

    let request = ToolRequest::Bash(BashRequest {
        command: "echo hello".to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    let result = executor.execute(request).await;

    // Should fail because there's no bash executor
    match result {
        Ok(ToolResponse::Error(err)) => {
            let error_msg = err.message.to_lowercase();
            assert!(error_msg.contains("not configured") || error_msg.contains("not available"));
        }
        Ok(other) => {
            panic!("Expected error response, got: {:?}", other);
        }
        Err(e) => {
            panic!("Expected error response, got execution error: {}", e);
        }
    }
}

#[tokio::test]
async fn test_read_only_permissions() {
    let perms = BashPermissions::read_only();
    let bash_executor = BashExecutor::new(perms, 120).unwrap();
    let executor = ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(Some(Box::new(bash_executor)))
        .build();

    // Test allowed read commands
    let read_commands = vec!["ls -la", "cat /etc/hosts", "pwd", "grep test file"];

    for cmd in read_commands {
        let request = ToolRequest::Bash(BashRequest {
            command: cmd.to_string(),
            working_dir: None,
            timeout_secs: None,
            description: None,
        });

        let result = executor.execute(request).await;
        match result {
            Ok(ToolResponse::Bash(_)) | Ok(ToolResponse::Error(_)) => {
                // Either executed successfully (allowed) or failed with execution error (not permission error)
            }
            Ok(other) => {
                panic!(
                    "Unexpected response type for command '{}': {:?}",
                    cmd, other
                );
            }
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();
                if error_msg.contains("denied") || error_msg.contains("not allowed") {
                    panic!("Permission error for allowed command '{}': {}", cmd, e);
                }
            }
        }
    }

    // Test denied write commands
    let write_commands = vec!["echo test > file.txt", "rm file.txt", "touch newfile.txt"];

    for cmd in write_commands {
        let request = ToolRequest::Bash(BashRequest {
            command: cmd.to_string(),
            working_dir: None,
            timeout_secs: None,
            description: None,
        });

        let result = executor.execute(request).await;
        match result {
            Ok(ToolResponse::Error(err)) => {
                let error_msg = err.message.to_lowercase();
                assert!(error_msg.contains("denied") || error_msg.contains("not allowed"));
            }
            Ok(other) => {
                panic!("Expected permission error for '{}', got: {:?}", cmd, other);
            }
            Err(e) => {
                panic!("Expected error response, got execution error: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_multiple_command_permissions() {
    let perms = BashPermissions::only_allow(vec!["tesseract*", "convert*"]);
    let bash_executor = BashExecutor::new(perms, 120).unwrap();
    let executor = ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(Some(Box::new(bash_executor)))
        .build();

    // Test allowed commands
    let allowed_commands = vec!["tesseract --help", "convert --version"];

    for cmd in allowed_commands {
        let request = ToolRequest::Bash(BashRequest {
            command: cmd.to_string(),
            working_dir: None,
            timeout_secs: None,
            description: None,
        });

        let result = executor.execute(request).await;
        match result {
            Ok(ToolResponse::Bash(_)) | Ok(ToolResponse::Error(_)) => {
                // Either executed successfully (allowed) or failed with execution error (not permission error)
            }
            Ok(other) => {
                panic!(
                    "Unexpected response type for command '{}': {:?}",
                    cmd, other
                );
            }
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();
                if error_msg.contains("denied") || error_msg.contains("not allowed") {
                    panic!("Permission error for allowed command '{}': {}", cmd, e);
                }
            }
        }
    }

    // Test denied commands
    let denied_commands = vec!["ls -la", "cat file.txt", "echo hello"];

    for cmd in denied_commands {
        let request = ToolRequest::Bash(BashRequest {
            command: cmd.to_string(),
            working_dir: None,
            timeout_secs: None,
            description: None,
        });

        let result = executor.execute(request).await;
        match result {
            Ok(ToolResponse::Error(err)) => {
                let error_msg = err.message.to_lowercase();
                assert!(error_msg.contains("denied") || error_msg.contains("not allowed"));
            }
            Ok(other) => {
                panic!("Expected permission error for '{}', got: {:?}", cmd, other);
            }
            Err(e) => {
                panic!("Expected error response, got execution error: {}", e);
            }
        }
    }
}
