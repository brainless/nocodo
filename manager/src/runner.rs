use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};

use crate::database::Database;
use crate::models::AiSession;
use crate::websocket::WebSocketBroadcaster;

/// In-process runner that executes AI tool sessions with piped stdio and streams output
pub struct Runner {
    db: Arc<Database>,
    ws: Arc<WebSocketBroadcaster>,
    inputs: Mutex<HashMap<String, mpsc::Sender<String>>>, // session_id -> stdin tx
}

impl Runner {
    pub fn new(db: Arc<Database>, ws: Arc<WebSocketBroadcaster>) -> Self {
        Self {
            db,
            ws,
            inputs: Mutex::new(HashMap::new()),
        }
    }

    /// Start executing a session in the background. Returns Ok(()) when spawned successfully.
    pub async fn start_session(
        &self,
        session: AiSession,
        enhanced_prompt: String,
    ) -> anyhow::Result<()> {
        let session_id = session.id.clone();
        let tool = session.tool_name.clone();

        tracing::info!(
            "Runner: Starting session {} with tool '{}'",
            session_id,
            tool
        );

        // Prepare command mapping and args
        let (cmd_name, args, prompt_file) = Self::build_command_args(&tool, &enhanced_prompt)?;

        tracing::info!("Runner: Command to execute: {} {:?}", cmd_name, args);

        let mut cmd = Command::new(&cmd_name);
        cmd.args(args);

        tracing::info!("Runner: Command configured for session {}", session_id);

        // If this session is associated with a project via its work, run the tool in that project's directory
        if let Ok(work) = self.db.get_work_by_id(&session.work_id) {
            if let Some(ref project_id) = work.project_id {
                if let Ok(project) = self.db.get_project_by_id(project_id) {
                    let project_dir = std::path::Path::new(&project.path);
                    if project_dir.exists() {
                        cmd.current_dir(project_dir);
                    } else {
                        // Best-effort: record a hint in outputs to help diagnostics
                        let _ = self.db.create_ai_session_output(
                            &session_id,
                            &format!(
                                "[nocodo runner] Warning: Project directory not found: {path}. Running in Manager's CWD.",
                                path = project.path
                            ),
                        );
                    }
                } else {
                    let _ = self.db.create_ai_session_output(
                        &session_id,
                        &format!(
                            "[nocodo runner] Warning: Unable to load project for id {project_id}. Running in Manager's CWD."
                        ),
                    );
                }
            }
        }

        // Configure stdio - Claude --print doesn't need stdin, other tools might
        let needs_stdin = tool != "claude" || (cmd_name == "sh"); // sh for multi-line prompts needs stdin
        cmd.stdin(if needs_stdin {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                let _ = self.db.create_ai_session_output(
                    &session_id,
                    &format!("Failed to start tool '{cmd_name}': {e}"),
                );
                self.mark_failed(&session_id).await.ok();
                return Err(anyhow::anyhow!(e));
            }
        };

        // Stdout reader
        if let Some(stdout) = child.stdout.take() {
            tracing::info!(
                "Runner: Setting up stdout reader for session {}",
                session_id
            );
            let ws = Arc::clone(&self.ws);
            let db = Arc::clone(&self.db);
            let sid = session_id.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                let mut seq: u64 = 0;
                tracing::info!("Runner: Started stdout reader for session {}", sid);
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::info!("Runner: Got stdout line for session {}: {}", sid, line);
                    ws.broadcast_ai_output_chunk(sid.clone(), "stdout", &line, seq);
                    let _ = db.create_ai_session_output(&sid, &line);
                    seq = seq.saturating_add(1);
                }
                tracing::info!("Runner: Stdout reader finished for session {}", sid);
            });
        } else {
            tracing::warn!("Runner: No stdout available for session {}", session_id);
        }

        // Stderr reader
        if let Some(stderr) = child.stderr.take() {
            let ws = Arc::clone(&self.ws);
            let db = Arc::clone(&self.db);
            let sid = session_id.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                let mut seq: u64 = 0;
                while let Ok(Some(line)) = lines.next_line().await {
                    ws.broadcast_ai_output_chunk(sid.clone(), "stderr", &line, seq);
                    let _ = db.create_ai_session_output(&sid, &line);
                    seq = seq.saturating_add(1);
                }
            });
        }

        // Stdin input channel
        if let Some(mut stdin) = child.stdin.take() {
            let (tx, mut rx) = mpsc::channel::<String>(128);
            self.inputs.lock().await.insert(session_id.clone(), tx);
            tokio::spawn(async move {
                while let Some(content) = rx.recv().await {
                    let _ = stdin.write_all(content.as_bytes()).await;
                    let _ = stdin.write_all(b"\n").await;
                    let _ = stdin.flush().await;
                }
            });
        }

        // Waiter
        let db = Arc::clone(&self.db);
        let ws = Arc::clone(&self.ws);
        let sid = session_id.clone();
        tokio::spawn(async move {
            let status = child.wait().await;
            // Clean up temp prompt file, if any
            if let Some(pf) = prompt_file {
                let _ = std::fs::remove_file(pf);
            }
            match status {
                Ok(s) => {
                    if s.success() {
                        // Mark completed
                        let _ = Self::mark_completed_static(&db, &ws, &sid).await;
                    } else {
                        // Mark failed
                        let _ = Self::mark_failed_static(&db, &ws, &sid).await;
                    }
                }
                Err(_) => {
                    let _ = Self::mark_failed_static(&db, &ws, &sid).await;
                }
            }
        });

        Ok(())
    }

    fn build_command_args(
        tool: &str,
        prompt: &str,
    ) -> anyhow::Result<(String, Vec<String>, Option<PathBuf>)> {
        let t = tool.to_lowercase();
        let cmd = match t.as_str() {
            "claude" | "claude-code" => "claude".to_string(),
            "gemini" | "gemini-cli" => "gemini".to_string(),
            "openai" | "openai-cli" => "openai".to_string(),
            // Qwen Code CLI binary is `qwen`
            "qwen" | "qwen-code" => "qwen".to_string(),
            _ => t,
        };

        // Some tools prefer a prompt file
        if cmd == "gemini" {
            let mut path = std::env::temp_dir();
            path.push(format!("nocodo_prompt_{}.txt", std::process::id()));
            std::fs::write(&path, prompt)?;
            return Ok((
                cmd,
                vec!["--input".to_string(), path.to_string_lossy().to_string()],
                Some(path),
            ));
        }

        // Claude supports --print with inline prompt, but use file for multi-line prompts
        if cmd == "claude" {
            // If prompt contains newlines, use a temp file approach to avoid shell escaping issues
            if prompt.contains('\n') {
                let mut path = std::env::temp_dir();
                path.push(format!("nocodo_claude_prompt_{}.txt", std::process::id()));
                std::fs::write(&path, prompt)?;
                // Use stdin approach: claude --print < prompt_file
                return Ok((
                    "sh".to_string(),
                    vec![
                        "-c".to_string(),
                        format!("claude --print < {}", path.to_string_lossy()),
                    ],
                    Some(path),
                ));
            } else {
                return Ok((cmd, vec!["--print".to_string(), prompt.to_string()], None));
            }
        }

        // Qwen Code expects --prompt <text>
        if cmd == "qwen" {
            return Ok((cmd, vec!["--prompt".to_string(), prompt.to_string()], None));
        }

        // Generic: pass prompt as a single arg
        Ok((cmd, vec![prompt.to_string()], None))
    }

    async fn mark_failed(&self, session_id: &str) -> anyhow::Result<()> {
        Self::mark_failed_static(&self.db, &self.ws, session_id).await
    }

    async fn mark_completed_static(
        db: &Arc<Database>,
        ws: &Arc<WebSocketBroadcaster>,
        session_id: &str,
    ) -> anyhow::Result<()> {
        let mut session = db.get_ai_session_by_id(session_id)?;
        session.complete();
        db.update_ai_session(&session)?;
        ws.broadcast_ai_session_completed(session_id.to_string());
        Ok(())
    }

    async fn mark_failed_static(
        db: &Arc<Database>,
        ws: &Arc<WebSocketBroadcaster>,
        session_id: &str,
    ) -> anyhow::Result<()> {
        let mut session = db.get_ai_session_by_id(session_id)?;
        session.fail();
        db.update_ai_session(&session)?;
        ws.broadcast_ai_session_failed(session_id.to_string());
        Ok(())
    }
}
