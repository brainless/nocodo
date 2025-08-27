use crate::error::CliError;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, error, info};

// Keep SocketRequest enums in sync with manager/src/socket.rs

#[derive(Debug, Serialize, Deserialize)]
pub enum SocketRequest {
    // Health and identity
    Ping,
    Identify {
        client_id: String,
        token: Option<String>,
    },

    // Sessions and project context
    CreateAiSession(CreateAiSessionRequest),
    GetProjectContext {
        project_path: String,
    },
    GetProjectByPath {
        project_path: String,
    },
    CompleteAiSession {
        session_id: String,
    },
    FailAiSession {
        session_id: String,
    },
    // New: record one-shot AI output for a session
    RecordAiOutput {
        session_id: String,
        output: String,
    },
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub path: Option<String>,
    pub language: Option<String>,
    pub framework: Option<String>,
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddExistingProjectRequest {
    pub name: String,
    pub path: String, // Required - must be existing directory
    pub language: Option<String>,
    pub framework: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub language: Option<String>,
    pub framework: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectResponse {
    pub project: Project,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub name: String,
    pub description: String,
    pub language: String,
    pub framework: Option<String>,
    pub files: Vec<TemplateFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
    pub executable: bool,
}

// Work history models
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageContentType {
    Text,
    Markdown,
    Json,
    Code { language: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageAuthorType {
    User,
    Ai,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkMessage {
    pub id: String,
    pub work_id: String,
    pub content: String,
    pub content_type: MessageContentType,
    pub author_type: MessageAuthorType,
    pub author_id: Option<String>,
    pub sequence_order: i32,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Work {
    pub id: String,
    pub title: String,
    pub project_id: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkWithHistory {
    pub work: Work,
    pub messages: Vec<WorkMessage>,
    pub total_messages: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWorkRequest {
    pub title: String,
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddMessageRequest {
    pub content: String,
    pub content_type: MessageContentType,
    pub author_type: MessageAuthorType,
    pub author_id: Option<String>,
}

pub struct ManagerClient {
    socket_path: String,
    http_client: reqwest::Client,
    manager_url: String,
}

impl ManagerClient {
    pub fn new(socket_path: String, manager_url: Option<String>) -> Self {
        let manager_url = manager_url.unwrap_or_else(|| "http://localhost:8081".to_string());
        Self {
            socket_path,
            http_client: reqwest::Client::new(),
            manager_url,
        }
    }

    pub async fn send_request(&self, request: SocketRequest) -> Result<SocketResponse, CliError> {
        debug!("Connecting to Manager daemon at: {}", self.socket_path);

        let stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            error!("Failed to connect to Manager daemon: {}", e);
            CliError::Communication(format!(
                "Cannot connect to Manager daemon at {}: {e}. Make sure nocodo-manager is running.",
                self.socket_path
            ))
        })?;

        let request_json = serde_json::to_string(&request)
            .map_err(|e| CliError::Communication(format!("Failed to serialize request: {e}")))?;

        debug!("Sending request: {}", request_json);

        let (reader, mut writer) = stream.into_split();

        writer
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| CliError::Communication(format!("Failed to write to socket: {e}")))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| CliError::Communication(format!("Failed to write newline: {e}")))?;

        let mut reader = BufReader::new(reader);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .await
            .map_err(|e| CliError::Communication(format!("Failed to read response: {e}")))?;

        debug!("Received response: {}", response_line.trim());

        let response: SocketResponse = serde_json::from_str(response_line.trim())
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        Ok(response)
    }

    pub async fn create_ai_session(
        &self,
        tool_name: String,
        prompt: String,
        project_path: Option<String>,
    ) -> Result<AiSession, CliError> {
        info!("Creating AI session for tool: {}", tool_name);

        // First, try to get project info if we have a project path
        let project_id = if let Some(ref path) = project_path {
            match self.get_project_by_path(path.clone()).await {
                Ok(project) => {
                    info!(
                        "Found existing project: {}",
                        project["name"].as_str().unwrap_or("Unknown")
                    );
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
                let session: AiSession = serde_json::from_value(data).map_err(|e| {
                    CliError::Communication(format!("Failed to parse session data: {e}"))
                })?;

                info!("Created AI session: {}", session.id);
                Ok(session)
            }
            SocketResponse::Error { message } => {
                error!("Failed to create AI session: {}", message);
                Err(CliError::Communication(format!("Manager error: {message}")))
            }
        }
    }

    pub async fn get_project_context(&self, project_path: String) -> Result<String, CliError> {
        let request = SocketRequest::GetProjectContext { project_path };
        let response = self.send_request(request).await?;

        match response {
            SocketResponse::Success { data } => {
                let context = data["context"].as_str().ok_or_else(|| {
                    CliError::Communication("Invalid context response".to_string())
                })?;
                Ok(context.to_string())
            }
            SocketResponse::Error { message } => {
                Err(CliError::Communication(format!("Manager error: {message}")))
            }
        }
    }

    pub async fn get_project_by_path(
        &self,
        project_path: String,
    ) -> Result<serde_json::Value, CliError> {
        let request = SocketRequest::GetProjectByPath { project_path };
        let response = self.send_request(request).await?;

        match response {
            SocketResponse::Success { data } => Ok(data),
            SocketResponse::Error { message } => {
                Err(CliError::Communication(format!("Manager error: {message}")))
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
                Err(CliError::Communication(format!("Manager error: {message}")))
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
                Err(CliError::Communication(format!("Manager error: {message}")))
            }
        }
    }

    pub async fn record_ai_output(
        &self,
        session_id: String,
        output: String,
    ) -> Result<(), CliError> {
        info!(
            "Recording AI output for session: {} ({} bytes)",
            session_id,
            output.len()
        );
        let request = SocketRequest::RecordAiOutput { session_id, output };
        let response = self.send_request(request).await?;
        match response {
            SocketResponse::Success { .. } => Ok(()),
            SocketResponse::Error { message } => {
                Err(CliError::Communication(format!("Manager error: {message}")))
            }
        }
    }

    // HTTP API methods

    pub async fn create_ai_session_http(
        &self,
        tool_name: &str,
        prompt: &str,
        project_path: String,
    ) -> Result<AiSession, CliError> {
        info!("Creating AI session via HTTP API for tool: {}", tool_name);

        // First, try to get project info if we have a project path
        let project_id = match self.get_project_by_http_path(project_path.clone()).await {
            Ok(project) => {
                info!(
                    "Found existing project: {}",
                    project["name"].as_str().unwrap_or("Unknown")
                );
                Some(project["id"].as_str().unwrap_or("").to_string())
            }
            Err(_) => {
                debug!("No existing project found for path: {}", project_path);
                None
            }
        };

        let request_body = serde_json::json!({
            "project_id": project_id,
            "tool_name": tool_name,
            "prompt": prompt
        });

        let url = format!("{}/api/ai/sessions", self.manager_url);
        debug!("POST {}", url);

        let response = self
            .http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| CliError::Communication(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Communication(format!(
                "HTTP {status} error: {error_text}"
            )));
        }

        let session_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        let session: AiSession = serde_json::from_value(session_response["session"].clone())
            .map_err(|e| CliError::Communication(format!("Failed to parse session data: {e}")))?;

        info!("Created AI session via HTTP API: {}", session.id);
        Ok(session)
    }

    async fn get_project_by_http_path(
        &self,
        project_path: String,
    ) -> Result<serde_json::Value, CliError> {
        let url = format!("{}/api/projects", self.manager_url);
        debug!("GET {}", url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| CliError::Communication(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Communication(format!(
                "HTTP {status} error: {error_text}"
            )));
        }

        let projects_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        let projects = projects_response["projects"].as_array().ok_or_else(|| {
            CliError::Communication("Invalid projects response format".to_string())
        })?;

        for project in projects {
            if let Some(path) = project["path"].as_str() {
                if path == project_path {
                    return Ok(project.clone());
                }
            }
        }

        Err(CliError::Communication(format!(
            "No project found for path: {project_path}"
        )))
    }

    pub async fn create_project(&self, request: CreateProjectRequest) -> Result<Project, CliError> {
        info!("Creating project '{}' via HTTP API", request.name);

        let url = format!("{}/api/projects", self.manager_url);
        debug!("POST {}", url);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| CliError::Communication(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Communication(format!(
                "HTTP {status} error: {error_text}"
            )));
        }

        let project_response: ProjectResponse = response
            .json()
            .await
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        info!(
            "Project '{}' created successfully at {}",
            project_response.project.name, project_response.project.path
        );
        Ok(project_response.project)
    }

    pub async fn add_existing_project(
        &self,
        request: AddExistingProjectRequest,
    ) -> Result<Project, CliError> {
        info!("Adding existing project '{}' via HTTP API", request.name);

        let url = format!("{}/api/projects/add-existing", self.manager_url);
        debug!("POST {}", url);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| CliError::Communication(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Communication(format!(
                "HTTP {status} error: {error_text}"
            )));
        }

        let project_response: ProjectResponse = response
            .json()
            .await
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        info!(
            "Existing project '{}' registered successfully at {}",
            project_response.project.name, project_response.project.path
        );
        Ok(project_response.project)
    }

    pub async fn get_templates(&self) -> Result<Vec<ProjectTemplate>, CliError> {
        info!("Fetching available templates via HTTP API");

        let url = format!("{}/api/templates", self.manager_url);
        debug!("GET {}", url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| CliError::Communication(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Communication(format!(
                "HTTP {status} error: {error_text}"
            )));
        }

        let templates: Vec<ProjectTemplate> = response
            .json()
            .await
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        info!("Fetched {} templates", templates.len());
        Ok(templates)
    }

    pub async fn check_manager_status(&self) -> Result<bool, CliError> {
        let url = format!("{}/api/health", self.manager_url);
        debug!("Checking Manager daemon status at: {}", url);

        match self.http_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!("Manager daemon is responsive");
                    Ok(true)
                } else {
                    debug!(
                        "Manager daemon returned error status: {}",
                        response.status()
                    );
                    Ok(false)
                }
            }
            Err(e) => {
                debug!("Failed to connect to Manager daemon: {}", e);
                Ok(false)
            }
        }
    }

    // Work management HTTP methods
    pub async fn create_work(&self, title: String, project_id: Option<String>) -> Result<Work, CliError> {
        info!("Creating work '{}' via HTTP API", title);

        let request_body = CreateWorkRequest { title, project_id };

        let url = format!("{}/api/works", self.manager_url);
        debug!("POST {}", url);

        let response = self
            .http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| CliError::Communication(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Communication(format!(
                "HTTP {status} error: {error_text}"
            )));
        }

        let work_response: Work = response
            .json()
            .await
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        info!("Work '{}' created successfully with ID: {}", work_response.title, work_response.id);
        Ok(work_response)
    }

    pub async fn list_works(&self) -> Result<Vec<Work>, CliError> {
        info!("Fetching works via HTTP API");

        let url = format!("{}/api/works", self.manager_url);
        debug!("GET {}", url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| CliError::Communication(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Communication(format!(
                "HTTP {status} error: {error_text}"
            )));
        }

        let works_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        let works: Vec<Work> = serde_json::from_value(works_response["works"].clone())
            .map_err(|e| CliError::Communication(format!("Failed to parse works data: {e}")))?;

        info!("Fetched {} works", works.len());
        Ok(works)
    }

    pub async fn get_work_with_history(&self, work_id: &str) -> Result<WorkWithHistory, CliError> {
        info!("Fetching work with history via HTTP API for work ID: {}", work_id);

        let url = format!("{}/api/works/{}", self.manager_url, work_id);
        debug!("GET {}", url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| CliError::Communication(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Communication(format!(
                "HTTP {status} error: {error_text}"
            )));
        }

        let work_with_history: WorkWithHistory = response
            .json()
            .await
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        info!("Fetched work with history for work ID: {}", work_id);
        Ok(work_with_history)
    }

    pub async fn add_message_to_work(
        &self,
        work_id: String,
        content: String,
        content_type: MessageContentType,
        author_type: MessageAuthorType,
        author_id: Option<String>,
    ) -> Result<WorkMessage, CliError> {
        info!("Adding message to work via HTTP API for work ID: {}", work_id);

        let request_body = AddMessageRequest {
            content,
            content_type,
            author_type,
            author_id,
        };

        let url = format!("{}/api/works/{}/messages", self.manager_url, work_id);
        debug!("POST {}", url);

        let response = self
            .http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| CliError::Communication(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Communication(format!(
                "HTTP {status} error: {error_text}"
            )));
        }

        let message_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CliError::Communication(format!("Failed to parse response: {e}")))?;

        let message: WorkMessage = serde_json::from_value(message_response["message"].clone())
            .map_err(|e| CliError::Communication(format!("Failed to parse message data: {e}")))?;

        info!("Message added to work ID: {}", work_id);
        Ok(message)
    }
}
