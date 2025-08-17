use crate::error::CliError;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, error, info};

#[derive(Debug, Serialize, Deserialize)]
pub enum SocketRequest {
    CreateAiSession(CreateAiSessionRequest),
    GetProjectContext { project_path: String },
    GetProjectByPath { project_path: String },
    CompleteAiSession { session_id: String },
    FailAiSession { session_id: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SocketResponse {
    Success { data: serde_json::Value },
    Error { message: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAiSessionRequest {
    pub project_id: Option<String>,
    pub tool_name: String,
    pub prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiSession {
    pub id: String,
    pub project_id: Option<String>,
    pub tool_name: String,
    pub status: String,
    pub prompt: String,
    pub project_context: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

pub struct ManagerClient {
    socket_path: String,
}

impl ManagerClient {
    pub fn new(socket_path: String) -> Self {
        Self { socket_path }
    }

    pub async fn send_request(&self, request: SocketRequest) -> Result<SocketResponse, CliError> {
        debug!("Connecting to Manager daemon at: {}", self.socket_path);
        
        let stream = UnixStream::connect(&self.socket_path).await
            .map_err(|e| {
                error!("Failed to connect to Manager daemon: {}", e);
                CliError::Communication(format!("Cannot connect to Manager daemon at {}: {}. Make sure nocodo-manager is running.", self.socket_path, e))
            })?;

        let request_json = serde_json::to_string(&request)
            .map_err(|e| CliError::Communication(format!("Failed to serialize request: {}", e)))?;
        
        debug!("Sending request: {}", request_json);
        
        let (reader, mut writer) = stream.into_split();
        
        writer.write_all(request_json.as_bytes()).await
            .map_err(|e| CliError::Communication(format!("Failed to write to socket: {}", e)))?;
        writer.write_all(b"\n").await
            .map_err(|e| CliError::Communication(format!("Failed to write newline: {}", e)))?;

        let mut reader = BufReader::new(reader);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await
            .map_err(|e| CliError::Communication(format!("Failed to read response: {}", e)))?;

        debug!("Received response: {}", response_line.trim());
        
        let response: SocketResponse = serde_json::from_str(&response_line.trim())
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {}", e)))?;

        Ok(response)
    }

    pub async fn create_ai_session(
        &self, 
        tool_name: String, 
        prompt: String,
        project_path: Option<String>
    ) -> Result<AiSession, CliError> {
        info!("Creating AI session for tool: {}", tool_name);
        
        // First, try to get project info if we have a project path
        let project_id = if let Some(ref path) = project_path {
            match self.get_project_by_path(path.clone()).await {
                Ok(project) => {
                    info!("Found existing project: {}", project["name"].as_str().unwrap_or("Unknown"));
                    Some(project["id"].as_str().unwrap_or("").to_string())
                }
                Err(_) => {
                    debug!("No existing project found for path: {}", path);
                    None
                }
            }
        } else {
            None
        };

        let request = SocketRequest::CreateAiSession(CreateAiSessionRequest {
            project_id,
            tool_name,
            prompt,
        });

        let response = self.send_request(request).await?;
        
        match response {
            SocketResponse::Success { data } => {
                let session: AiSession = serde_json::from_value(data)
                    .map_err(|e| CliError::Communication(format!("Failed to parse session data: {}", e)))?;
                
                info!("Created AI session: {}", session.id);
                Ok(session)
            }
            SocketResponse::Error { message } => {
                error!("Failed to create AI session: {}", message);
                Err(CliError::Communication(format!("Manager error: {}", message)))
            }
        }
    }

    pub async fn get_project_context(&self, project_path: String) -> Result<String, CliError> {
        let request = SocketRequest::GetProjectContext { project_path };
        let response = self.send_request(request).await?;
        
        match response {
            SocketResponse::Success { data } => {
                let context = data["context"].as_str()
                    .ok_or_else(|| CliError::Communication("Invalid context response".to_string()))?;
                Ok(context.to_string())
            }
            SocketResponse::Error { message } => {
                Err(CliError::Communication(format!("Manager error: {}", message)))
            }
        }
    }

    pub async fn get_project_by_path(&self, project_path: String) -> Result<serde_json::Value, CliError> {
        let request = SocketRequest::GetProjectByPath { project_path };
        let response = self.send_request(request).await?;
        
        match response {
            SocketResponse::Success { data } => Ok(data),
            SocketResponse::Error { message } => {
                Err(CliError::Communication(format!("Manager error: {}", message)))
            }
        }
    }

    pub async fn complete_ai_session(&self, session_id: String) -> Result<(), CliError> {
        info!("Completing AI session: {}", session_id);
        
        let request = SocketRequest::CompleteAiSession { session_id };
        let response = self.send_request(request).await?;
        
        match response {
            SocketResponse::Success { .. } => {
                info!("AI session completed successfully");
                Ok(())
            }
            SocketResponse::Error { message } => {
                error!("Failed to complete AI session: {}", message);
                Err(CliError::Communication(format!("Manager error: {}", message)))
            }
        }
    }

    pub async fn fail_ai_session(&self, session_id: String) -> Result<(), CliError> {
        info!("Marking AI session as failed: {}", session_id);
        
        let request = SocketRequest::FailAiSession { session_id };
        let response = self.send_request(request).await?;
        
        match response {
            SocketResponse::Success { .. } => {
                info!("AI session marked as failed");
                Ok(())
            }
            SocketResponse::Error { message } => {
                error!("Failed to fail AI session: {}", message);
                Err(CliError::Communication(format!("Manager error: {}", message)))
            }
        }
    }
}
