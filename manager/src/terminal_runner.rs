use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::database::Database;
use crate::models::{TerminalControlMessage, TerminalSession, ToolConfig};
use crate::websocket::WebSocketBroadcaster;

/// Maximum transcript size per session (20MB)
#[allow(dead_code)]
const MAX_TRANSCRIPT_SIZE: usize = 20 * 1024 * 1024;

/// Maximum session runtime (10 minutes)
#[allow(dead_code)]
const MAX_SESSION_RUNTIME: Duration = Duration::from_secs(10 * 60);

/// PTY-based terminal runner for interactive AI tools
pub struct TerminalRunner {
    db: Arc<Database>,
    #[allow(dead_code)]
    ws: Arc<WebSocketBroadcaster>,
    sessions: RwLock<HashMap<String, RunningSession>>,
    tool_registry: RwLock<HashMap<String, ToolConfig>>,
}

struct RunningSession {
    session: TerminalSession,
    input_tx: mpsc::Sender<TerminalControlMessage>,
    transcript: Arc<Mutex<Vec<u8>>>,
    _abort_handle: tokio::task::AbortHandle,
}

impl TerminalRunner {
    pub fn new(db: Arc<Database>, ws: Arc<WebSocketBroadcaster>) -> Self {
        Self {
            db,
            ws,
            sessions: RwLock::new(HashMap::new()),
            tool_registry: RwLock::new(Self::default_tool_registry()),
        }
    }

    /// Initialize with default tool registry
    fn default_tool_registry() -> HashMap<String, ToolConfig> {
        let mut registry = HashMap::new();

        // Claude Code tool
        registry.insert(
            "claude".to_string(),
            ToolConfig {
                name: "claude".to_string(),
                command: "claude".to_string(),
                args: vec!["--print".to_string()],
                requires_pty: true,
                working_dir: "project".to_string(),
                env: None,
            },
        );

        // Gemini CLI tool
        registry.insert(
            "gemini".to_string(),
            ToolConfig {
                name: "gemini".to_string(),
                command: "gemini".to_string(),
                args: vec!["--interactive".to_string()],
                requires_pty: true,
                working_dir: "project".to_string(),
                env: None,
            },
        );

        // Qwen Code tool
        registry.insert(
            "qwen".to_string(),
            ToolConfig {
                name: "qwen".to_string(),
                command: "qwen-code".to_string(),
                args: vec!["--interactive".to_string()],
                requires_pty: true,
                working_dir: "project".to_string(),
                env: None,
            },
        );

        registry
    }

    pub async fn get_tool_registry(&self) -> Vec<ToolConfig> {
        self.tool_registry.read().await.values().cloned().collect()
    }

    #[allow(dead_code)]
    pub async fn register_tool(&self, tool: ToolConfig) {
        self.tool_registry
            .write()
            .await
            .insert(tool.name.clone(), tool);
    }

    /// Start a new terminal session
    pub async fn start_session(
        &self,
        session: TerminalSession,
        _initial_prompt: Option<String>,
    ) -> anyhow::Result<()> {
        let session_id = session.id.clone();
        let tool_name = session.tool_name.clone();

        tracing::info!(
            "TerminalRunner: Starting PTY session {} with tool '{}'",
            session_id,
            tool_name
        );

        // Get tool configuration
        let tool_config = {
            let registry = self.tool_registry.read().await;
            registry
                .get(&tool_name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found in registry", tool_name))?
        };

        if !tool_config.requires_pty {
            return Err(anyhow::anyhow!(
                "Tool '{}' does not support PTY mode",
                tool_name
            ));
        }

        // Determine working directory
        let working_dir = if tool_config.working_dir == "project" {
            if let Ok(work) = self.db.get_work_by_id(&session.work_id) {
                if let Some(ref project_id) = work.project_id {
                    if let Ok(project) = self.db.get_project_by_id(project_id) {
                        Some(Path::new(&project.path).to_path_buf())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Spawn PTY session
        self.spawn_pty_session(session, tool_config, working_dir, _initial_prompt)
            .await
    }

    async fn spawn_pty_session(
        &self,
        session: TerminalSession,
        tool_config: ToolConfig,
        working_dir: Option<std::path::PathBuf>,
        _initial_prompt: Option<String>,
    ) -> anyhow::Result<()> {
        let _session_id = session.id.clone();

        // Create PTY system
        let pty_system = native_pty_system();
        let pty_pair = pty_system.openpty(PtySize {
            rows: session.rows,
            cols: session.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Build command
        let mut cmd = CommandBuilder::new(&tool_config.command);
        cmd.args(&tool_config.args);

        if let Some(dir) = working_dir {
            cmd.cwd(dir);
        }

        // Set environment variables
        if let Some(ref env) = tool_config.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Spawn child process
        let _child = pty_pair.slave.spawn_command(cmd)?;
        drop(pty_pair.slave);

        // Enable PTY functionality - uncomment the actual implementation
        // Get reader and writer from the master PTY
        let reader = pty_pair.master.try_clone_reader()?;
        let writer = pty_pair.master.try_clone_writer()?;

        // Create channels for communication
        let (input_tx, mut input_rx) = mpsc::channel::<TerminalControlMessage>(256);
        let transcript = Arc::new(Mutex::new(Vec::new()));

        // Store session info
        let running_session = {
            // Spawn main session task
            let db = Arc::clone(&self.db);
            let ws = Arc::clone(&self.ws);
            let session_clone = session.clone();
            let transcript_clone = Arc::clone(&transcript);
            let child_arc = Arc::new(Mutex::new(child));
            let session_id_clone = session_id.clone();

            let task = tokio::spawn(async move {
                // Send initial prompt if provided
                if let Some(prompt) = initial_prompt {
                    if let Err(e) = writer.write_all(prompt.as_bytes()).await {
                        tracing::error!("Failed to send initial prompt: {}", e);
                    }
                    if let Err(e) = writer.write_all(b"\n").await {
                        tracing::error!("Failed to send newline: {}", e);
                    }
                }

                let mut output_buffer = vec![0u8; 8192];
                let mut session_mut = session_clone;

                loop {
                    tokio::select! {
                        // Handle PTY output
                        result = reader.read(&mut output_buffer) => {
                            match result {
                                Ok(0) => {
                                    // EOF - process has exited
                                    break;
                                }
                                Ok(n) => {
                                    let output_bytes = &output_buffer[0..n];

                                    // Append to transcript
                                    {
                                        let mut transcript = transcript_clone.lock().await;
                                        transcript.extend_from_slice(output_bytes);

                                        // Trim transcript if it's too large
                                        if transcript.len() > MAX_TRANSCRIPT_SIZE {
                                            let excess = transcript.len() - MAX_TRANSCRIPT_SIZE;
                                            transcript.drain(0..excess);
                                        }
                                    }

                                    // Broadcast output as binary WebSocket frame
                                    ws.broadcast_terminal_output(
                                        session_id_clone.clone(),
                                        output_bytes.to_vec()
                                    ).await;
                                }
                                Err(e) => {
                                    tracing::error!("PTY read error: {}", e);
                                    break;
                                }
                            }
                        }

                        // Handle control messages
                        msg = input_rx.recv() => {
                            match msg {
                                Some(TerminalControlMessage::Input { data }) => {
                                    // Decode base64 input and write to PTY
                                    match base64::engine::general_purpose::STANDARD.decode(&data) {
                                        Ok(bytes) => {
                                            if let Err(e) = writer.write_all(&bytes).await {
                                                tracing::error!("Failed to write input to PTY: {}", e);
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to decode base64 input: {}", e);
                                        }
                                    }
                                }
                                Some(TerminalControlMessage::Resize { cols, rows }) => {
                                    // Resize PTY
                                    let new_size = PtySize {
                                        rows,
                                        cols,
                                        pixel_width: 0,
                                        pixel_height: 0,
                                    };
                                    if let Err(e) = pty_pair.master.resize(new_size) {
                                        tracing::error!("Failed to resize PTY: {}", e);
                                    } else {
                                        session_mut.resize(cols, rows);
                                        // Update session in database
                                        let _ = db.update_terminal_session(&session_mut);

                                        // Broadcast resize event
                                        ws.broadcast_terminal_control_message(
                                            session_id_clone.clone(),
                                            TerminalControlMessage::Resize { cols, rows }
                                        ).await;
                                    }
                                }
                                Some(TerminalControlMessage::Ping) => {
                                    // Send pong back
                                    ws.broadcast_terminal_control_message(
                                        session_id_clone.clone(),
                                        TerminalControlMessage::Pong
                                    ).await;
                                }
                                _ => {} // Ignore other message types
                            }
                        }

                        // Session timeout
                        _ = tokio::time::sleep(MAX_SESSION_RUNTIME) => {
                            tracing::warn!("Session {} timed out", session_id_clone);
                            break;
                        }
                    }
                }

                // Session ended - clean up
                let exit_code = {
                    let mut child_guard = child_arc.lock().await;
                    match timeout(Duration::from_secs(5), child_guard.wait()).await {
                        Ok(Ok(exit_status)) => exit_status.exit_code(),
                        Ok(Err(e)) => {
                            tracing::error!("Error waiting for child process: {}", e);
                            None
                        }
                        Err(_) => {
                            // Timeout - force kill
                            tracing::warn!("Child process didn't exit gracefully, force killing");
                            let _ = child_guard.kill();
                            None
                        }
                    }
                };

                // Update session status
                if exit_code == Some(0) {
                    session_mut.complete(exit_code);
                } else {
                    session_mut.fail(exit_code);
                }

                let _ = db.update_terminal_session(&session_mut);

                // Persist transcript
                let transcript_bytes = {
                    let transcript_guard = transcript_clone.lock().await;
                    transcript_guard.clone()
                };
                let _ = db.save_terminal_transcript(&session_id_clone, &transcript_bytes);

                // Broadcast session end
                ws.broadcast_terminal_control_message(
                    session_id_clone.clone(),
                    TerminalControlMessage::Status {
                        status: session_mut.status.clone(),
                        exit_code,
                    }
                ).await;
            });

            RunningSession {
                session,
                input_tx,
                transcript,
                _abort_handle: task.abort_handle(),
            }
        };

        // Store running session
        self.sessions.write().await.insert(session_id, running_session);

        tracing::info!("PTY session started successfully");
        Ok(())
    }

    /// Send input to a running session
    pub async fn send_input(
        &self,
        session_id: &str,
        message: TerminalControlMessage,
    ) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            session
                .input_tx
                .send(message)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send input: {}", e))
        } else {
            Err(anyhow::anyhow!("Session not found: {}", session_id))
        }
    }

    /// Get session info
    pub async fn get_session(&self, session_id: &str) -> Option<TerminalSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| s.session.clone())
    }

    /// List all active sessions
    #[allow(dead_code)]
    pub async fn list_sessions(&self) -> Vec<TerminalSession> {
        let sessions = self.sessions.read().await;
        sessions.values().map(|s| s.session.clone()).collect()
    }

    /// Terminate a session
    pub async fn terminate_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(running_session) = sessions.remove(session_id) {
            running_session._abort_handle.abort();
            tracing::info!("Terminated session: {}", session_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Session not found: {}", session_id))
        }
    }

    /// Get session transcript
    pub async fn get_transcript(&self, session_id: &str) -> Option<Vec<u8>> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let transcript = session.transcript.lock().await;
            Some(transcript.clone())
        } else {
            None
        }
    }
}
