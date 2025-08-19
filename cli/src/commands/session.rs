use crate::client::ManagerClient;
use crate::error::CliError;
use std::env;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Execute an AI coding session with enhanced context
pub async fn execute_ai_session(tool: &str, prompt: &str) -> Result<(), CliError> {
    info!(
        "Starting AI session with tool: {} and prompt: {}",
        tool, prompt
    );

    // Get current working directory as project path
    let project_path = env::current_dir()
        .map_err(|e| CliError::Command(format!("Failed to get current directory: {}", e)))?
        .to_string_lossy()
        .to_string();

    // Use default socket path for now - could be made configurable
    let socket_path = "/tmp/nocodo-manager.sock".to_string();
    let client = ManagerClient::new(socket_path, None);

    // Create AI session with Manager daemon
    let session = match client
        .create_ai_session(
            tool.to_string(),
            prompt.to_string(),
            Some(project_path.clone()),
        )
        .await
    {
        Ok(session) => {
            info!("Created AI session: {}", session.id);
            session
        }
        Err(e) => {
            warn!("Failed to create AI session with Manager: {}", e);
            warn!("Proceeding without Manager integration");

            // For now, continue without Manager integration during development
            // Create a mock session for logging purposes
            use crate::client::AiSession;
            AiSession {
                id: format!("mock-session-{}", std::process::id()),
                project_id: None,
                tool_name: tool.to_string(),
                status: "running".to_string(),
                prompt: prompt.to_string(),
                project_context: None,
                started_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                ended_at: None,
            }
        }
    };

    // Get project context from Manager if available
    let context = match client.get_project_context(project_path.clone()).await {
        Ok(ctx) => {
            info!("Retrieved project context from Manager");
            debug!("Context: {}", ctx);
            ctx
        }
        Err(e) => {
            warn!("Failed to get project context: {}", e);
            format!("Working directory: {}", project_path)
        }
    };

    // Build enhanced prompt with context
    let enhanced_prompt = format!(
        "Project Context:\n{}\n\nUser Request:\n{}\n\nInstructions: Use the `nocodo` command to get additional context about the project structure and to validate your changes.",
        context,
        prompt
    );

    info!("Executing {} with enhanced context", tool);
    debug!(
        "Enhanced prompt length: {} characters",
        enhanced_prompt.len()
    );

    // Execute the AI tool with the enhanced prompt and capture output
    let run_result = execute_ai_tool(tool, enhanced_prompt).await;

    // Build payload with actual captured stdout/stderr when available
    let (payload, status_ok) = match run_result {
        Ok(run) => {
            // Echo outputs to terminal (already handled inside execute_ai_tool previously; do it here now)
            if !run.stdout.is_empty() {
                println!("{}", run.stdout);
            }
            if !run.stderr.is_empty() {
                eprintln!("{}", run.stderr);
            }
            let p = serde_json::json!({
                "tool": tool,
                "prompt": prompt,
                "project_path": project_path,
                "success": run.success,
                "exit_code": run.exit_code,
                "stdout": run.stdout,
                "stderr": run.stderr,
            }).to_string();
            (p, true)
        }
        Err(err) => {
            let p = serde_json::json!({
                "tool": tool,
                "prompt": prompt,
                "project_path": project_path,
                "success": false,
                "error": err.to_string(),
                "stdout": "",
                "stderr": "",
            }).to_string();
            (p, false)
        }
    };

    if let Err(e) = client
        .record_ai_output(session.id.clone(), payload)
        .await
    {
        warn!("Failed to record AI output: {}", e);
    }

    // Mark session as completed or failed based on result
    if status_ok {
        info!("AI tool execution completed successfully");
        if let Err(e) = client.complete_ai_session(session.id.clone()).await {
            warn!("Failed to mark session as completed: {}", e);
        }
        Ok(())
    } else {
        error!("AI tool execution failed");
        if let Err(fail_err) = client.fail_ai_session(session.id.clone()).await {
            warn!("Failed to mark session as failed: {}", fail_err);
        }
        Err(CliError::Command("AI tool execution failed".to_string()))
    }
}

struct ToolRun {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    success: bool,
}

async fn execute_ai_tool(tool: , prompt: ) -> Result<ToolRun, CliError> {
    // Map tool names to actual commands
    let command = match tool.to_lowercase().as_str() {
        "claude" | "claude-code" => "claude",
        "gemini" | "gemini-cli" => "gemini",
        "openai" | "openai-cli" => "openai",
        _ => {
            // Try to use the tool name directly
            tool
        }
    };

    info!("Executing command: {}", command);

    // Check if the tool is available
    let tool_available = Command::new("which")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false);

    if !tool_available {
        return Err(CliError::Command(format!(
            "AI tool '{}' not found. Please ensure it's installed and in your PATH.",
            command
        )));
    }

    // Create a temporary file for the prompt
    let temp_dir = env::temp_dir();
    let prompt_file = temp_dir.join(format!("nocodo_prompt_{}.txt", std::process::id()));

    // Write prompt to temporary file
    std::fs::write(&prompt_file, prompt)
        .map_err(|e| CliError::Command(format!("Failed to write prompt file: {}", e)))?;

    info!("Wrote prompt to temporary file: {:?}", prompt_file);

    // Execute the AI tool with different approaches based on the tool
    let mut cmd = Command::new(command);

    match tool.to_lowercase().as_str() {
        "claude" | "claude-code" => {
            // Claude CLI accepts prompts directly as arguments
            // Use --print flag for non-interactive output
            cmd.arg("--print").arg(prompt);
        }
        "gemini" | "gemini-cli" => {
            // Gemini CLI might support different arguments
            cmd.arg("--input").arg(&prompt_file);
        }
        _ => {
            // Generic approach - try to pass the prompt directly
            cmd.arg(prompt);
        }
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| CliError::Command(format!("Failed to execute {}: {}", command, e)))?;

    // Clean up temporary file
    if let Err(e) = std::fs::remove_file(&prompt_file) {
        warn!("Failed to remove temporary prompt file: {}", e);
    }

    // Convert outputs to strings for capture
    let stdout_str = String::from_utf8_lossy(output.stdout).to_string();
    let stderr_str = String::from_utf8_lossy(output.stderr).to_string();

    let success = output.status.success();
    let exit_code = output.status.code();

    if success {
        info!("AI tool completed successfully");
    } else {
        let code = exit_code.unwrap_or(-1);
        info!("AI tool completed with non-zero exit: {}", code);
    }

    Ok(ToolRun {
        stdout: stdout_str,
        stderr: stderr_str,
        exit_code,
        success,
    })
}
