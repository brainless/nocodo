use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Execute bash commands with timeout and permission checking
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BashRequest {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl BashRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for command execution"
                },
                "timeout_secs": {
                    "type": "number",
                    "description": "Timeout in seconds (default: 120)",
                    "default": 120
                },
                "description": {
                    "type": "string",
                    "description": "Optional description of what the command does"
                }
            },
            "required": ["command"]
        })
    }
}

/// Bash command execution tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashResponse {
    pub command: String,
    pub working_dir: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
    pub execution_time_secs: f64,
}
