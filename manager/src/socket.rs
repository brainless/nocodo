use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::models::{
    AddMessageRequest, AiSession, CreateAiSessionRequest, CreateWorkRequest, Project,
};
use crate::websocket::WebSocketBroadcaster;
use serde::{Deserialize, Serialize};
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio_stream::wrappers::UnixListenerStream;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};

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

    // Work history management
    CreateWork(CreateWorkRequest),
    GetWorkWithHistory {
        work_id: String,
    },
    ListWorks,
    AddMessageToWork(AddMessageRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SocketResponse {
    Success { data: serde_json::Value },
    Error { message: String },
}

pub struct SocketServer {
    listener: UnixListener,
    database: Arc<Database>,
    ws_broadcaster: Arc<WebSocketBroadcaster>,
}

impl SocketServer {
    pub async fn new(
        socket_path: &str,
        database: Arc<Database>,
        ws_broadcaster: Arc<WebSocketBroadcaster>,
    ) -> AppResult<Self> {
        // Remove existing socket file if it exists
        if Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path)?;
        }

        let listener = UnixListener::bind(socket_path)
            .map_err(|e| AppError::Internal(format!("Failed to bind Unix socket: {e}")))?;

        // Restrict socket permissions to 600 (owner read/write)
        if let Err(e) = std::fs::set_permissions(socket_path, Permissions::from_mode(0o600)) {
            warn!("Failed to set socket permissions on {}: {}", socket_path, e);
        }

        info!("Unix socket server listening on: {}", socket_path);

        Ok(SocketServer {
            listener,
            database,
            ws_broadcaster,
        })
    }

    pub async fn run(self) -> AppResult<()> {
        let mut listener_stream = UnixListenerStream::new(self.listener);

        info!("Socket server started, waiting for connections...");

        while let Some(stream) = listener_stream.next().await {
            match stream {
                Ok(stream) => {
                    let db = Arc::clone(&self.database);
                    let ws_broadcaster = Arc::clone(&self.ws_broadcaster);
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, db, ws_broadcaster).await {
                            error!("Error handling socket connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    warn!("Failed to accept socket connection: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(
        stream: UnixStream,
        database: Arc<Database>,
        ws_broadcaster: Arc<WebSocketBroadcaster>,
    ) -> AppResult<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                warn!("Client disconnected without sending data");
                return Ok(());
            }
            Ok(_) => {
                let request: SocketRequest = serde_json::from_str(line.trim()).map_err(|e| {
                    AppError::Internal(format!("Failed to parse socket request: {e}"))
                })?;

                let response = Self::process_request(request, &database, &ws_broadcaster).await;
                let response_json = serde_json::to_string(&response).map_err(|e| {
                    AppError::Internal(format!("Failed to serialize response: {e}"))
                })?;

                writer
                    .write_all(response_json.as_bytes())
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to write response: {e}")))?;
                writer
                    .write_all(b"\n")
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to write newline: {e}")))?;
            }
            Err(e) => {
                error!("Failed to read from socket: {}", e);
                return Err(AppError::Internal(format!("Socket read error: {e}")));
            }
        }

        Ok(())
    }

    async fn process_request(
        request: SocketRequest,
        database: &Database,
        ws_broadcaster: &WebSocketBroadcaster,
    ) -> SocketResponse {
        match request {
            SocketRequest::Ping => {
                let data = serde_json::json!({ "ok": true });
                SocketResponse::Success { data }
            }

            SocketRequest::Identify { client_id, token } => {
                info!("Client identified: {}", client_id);
                // For MVP: accept any token if provided; log presence
                let data = serde_json::json!({
                    "client_id": client_id,
                    "authenticated": token.is_some(),
                });
                SocketResponse::Success { data }
            }

            SocketRequest::CreateAiSession(req) => {
                info!("Creating AI session for tool: {}", req.tool_name);

                // Validate that work and message exist
                let work = match database.get_work_by_id(&req.work_id) {
                    Ok(work) => work,
                    Err(e) => {
                        error!("Failed to get work {}: {}", req.work_id, e);
                        let message = format!("Failed to get work: {e}");
                        return SocketResponse::Error { message };
                    }
                };

                let messages = match database.get_work_messages(&req.work_id) {
                    Ok(messages) => messages,
                    Err(e) => {
                        error!("Failed to get messages for work {}: {}", req.work_id, e);
                        let message = format!("Failed to get messages: {e}");
                        return SocketResponse::Error { message };
                    }
                };

                if !messages.iter().any(|m| m.id == req.message_id) {
                    error!(
                        "Message {} not found in work {}",
                        req.message_id, req.work_id
                    );
                    let message = "Message not found in work".to_string();
                    return SocketResponse::Error { message };
                }

                // Get project context if work is associated with a project
                let project_context = if let Some(ref project_id) = work.project_id {
                    match database.get_project_by_id(project_id) {
                        Ok(project) => Some(Self::generate_project_context(&project)),
                        Err(e) => {
                            warn!("Failed to get project context for {}: {}", project_id, e);
                            None
                        }
                    }
                } else {
                    None
                };

                let session = AiSession::new(
                    req.work_id.clone(),
                    req.message_id.clone(),
                    req.tool_name.clone(),
                    project_context,
                );

                match database.create_ai_session(&session) {
                    Ok(()) => {
                        // Broadcast AI session creation via WebSocket
                        ws_broadcaster.broadcast_ai_session_created(session.clone());

                        let data = serde_json::to_value(&session).unwrap_or_default();
                        SocketResponse::Success { data }
                    }
                    Err(e) => {
                        error!("Failed to create AI session: {}", e);
                        SocketResponse::Error {
                            message: format!("Failed to create AI session: {e}"),
                        }
                    }
                }
            }

            SocketRequest::GetProjectContext { project_path } => {
                info!("Getting project context for path: {}", project_path);

                // For now, we'll generate basic context based on the path
                // In the future, this could analyze the project structure
                let context = Self::generate_path_context(&project_path);
                let data = serde_json::json!({ "context": context });
                SocketResponse::Success { data }
            }

            SocketRequest::GetProjectByPath { project_path } => {
                info!("Getting project by path: {}", project_path);

                // Try to find project by path in database
                match database.get_all_projects() {
                    Ok(projects) => {
                        let matching_project =
                            projects.into_iter().find(|p| p.path == project_path);

                        match matching_project {
                            Some(project) => {
                                let data = serde_json::to_value(&project).unwrap_or_default();
                                SocketResponse::Success { data }
                            }
                            None => SocketResponse::Error {
                                message: format!("No project found for path: {project_path}"),
                            },
                        }
                    }
                    Err(e) => {
                        error!("Failed to get projects: {}", e);
                        SocketResponse::Error {
                            message: format!("Database error: {e}"),
                        }
                    }
                }
            }

            SocketRequest::CompleteAiSession { session_id } => {
                info!("Completing AI session: {}", session_id);

                match database.get_ai_session_by_id(&session_id) {
                    Ok(mut session) => {
                        session.complete();
                        match database.update_ai_session(&session) {
                            Ok(()) => {
                                // Broadcast AI session completion via WebSocket
                                ws_broadcaster.broadcast_ai_session_completed(session.id.clone());

                                let data = serde_json::to_value(&session).unwrap_or_default();
                                SocketResponse::Success { data }
                            }
                            Err(e) => {
                                error!("Failed to update AI session: {}", e);
                                SocketResponse::Error {
                                    message: format!("Failed to complete session: {e}"),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("AI session not found: {}", e);
                        SocketResponse::Error {
                            message: format!("Session not found: {session_id}"),
                        }
                    }
                }
            }

            SocketRequest::FailAiSession { session_id } => {
                info!("Marking AI session as failed: {}", session_id);

                match database.get_ai_session_by_id(&session_id) {
                    Ok(mut session) => {
                        session.fail();
                        match database.update_ai_session(&session) {
                            Ok(()) => {
                                // Broadcast AI session failure via WebSocket
                                ws_broadcaster.broadcast_ai_session_failed(session.id.clone());

                                let data = serde_json::to_value(&session).unwrap_or_default();
                                SocketResponse::Success { data }
                            }
                            Err(e) => {
                                error!("Failed to update AI session: {}", e);
                                SocketResponse::Error {
                                    message: format!("Failed to fail session: {e}"),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("AI session not found: {}", e);
                        SocketResponse::Error {
                            message: format!("Session not found: {session_id}"),
                        }
                    }
                }
            }

            SocketRequest::RecordAiOutput { session_id, output } => {
                info!(
                    "Recording AI output for session: {} ({} bytes)",
                    session_id,
                    output.len()
                );

                // Ensure session exists
                match database.get_ai_session_by_id(&session_id) {
                    Ok(_session) => match database.create_ai_session_output(&session_id, &output) {
                        Ok(()) => {
                            let data = serde_json::json!({ "ok": true, "session_id": session_id });
                            SocketResponse::Success { data }
                        }
                        Err(e) => {
                            error!("Failed to record AI output: {}", e);
                            SocketResponse::Error {
                                message: format!("Failed to record output: {e}"),
                            }
                        }
                    },
                    Err(e) => {
                        error!("AI session not found for recording output: {}", e);
                        SocketResponse::Error {
                            message: format!("Session not found: {session_id}"),
                        }
                    }
                }
            }

            // Work history management requests
            SocketRequest::CreateWork(req) => {
                info!("Creating work: {}", req.title);

                // Create the work object
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| AppError::Internal(format!("Failed to get timestamp: {e}")))
                    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                    .as_secs() as i64;

                let work = crate::models::Work {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: req.title,
                    project_id: req.project_id,
                    status: "active".to_string(),
                    created_at: now,
                    updated_at: now,
                };

                match database.create_work(&work) {
                    Ok(()) => {
                        let data = serde_json::to_value(&work).unwrap_or_default();
                        SocketResponse::Success { data }
                    }
                    Err(e) => {
                        error!("Failed to create work: {}", e);
                        SocketResponse::Error {
                            message: format!("Failed to create work: {e}"),
                        }
                    }
                }
            }

            SocketRequest::GetWorkWithHistory { work_id } => {
                info!("Getting work with history: {}", work_id);

                match database.get_work_with_messages(&work_id) {
                    Ok(work_with_history) => {
                        let data = serde_json::to_value(&work_with_history).unwrap_or_default();
                        SocketResponse::Success { data }
                    }
                    Err(e) => {
                        error!("Failed to get work with history: {}", e);
                        SocketResponse::Error {
                            message: format!("Failed to get work: {e}"),
                        }
                    }
                }
            }

            SocketRequest::ListWorks => {
                info!("Listing all works");

                match database.get_all_works() {
                    Ok(works) => {
                        let data = serde_json::json!({ "works": works });
                        SocketResponse::Success { data }
                    }
                    Err(e) => {
                        error!("Failed to list works: {}", e);
                        SocketResponse::Error {
                            message: format!("Failed to list works: {e}"),
                        }
                    }
                }
            }

            SocketRequest::AddMessageToWork(_req) => {
                info!("Adding message to work");

                // For now, we'll need to determine the work_id from context or add it to the request
                // This is a simplified implementation - in a real scenario, we'd need the work_id
                SocketResponse::Error {
                    message: "AddMessageToWork via socket not fully implemented".to_string(),
                }
            }
        }
    }

    fn generate_project_context(project: &Project) -> String {
        format!(
            "Project: {}\nPath: {}\nLanguage: {}\nFramework: {}\nStatus: {}",
            project.name,
            project.path,
            project.language.as_deref().unwrap_or("Unknown"),
            project.framework.as_deref().unwrap_or("None"),
            project.status
        )
    }

    fn generate_path_context(project_path: &str) -> String {
        // Basic context generation - in the future this could analyze the project structure
        let path = Path::new(project_path);
        let project_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");

        format!("Working directory: {project_path}\nProject name (inferred): {project_name}")
    }
}
