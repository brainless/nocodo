use crate::models::{AiSession, Project, TerminalControlMessage};
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use base64::{self, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use ts_rs::TS;
use uuid::Uuid;

/// WebSocket message types for real-time communication
#[derive(Debug, Clone, Serialize, Deserialize, TS, Message)]
#[ts(export)]
#[serde(tag = "type", content = "payload")]
#[rtype(result = "()")]
pub enum WebSocketMessage {
    // System messages
    Connected {
        client_id: String,
    },
    Disconnected {
        client_id: String,
    },

    // Project updates
    ProjectCreated {
        project: Project,
    },
    ProjectUpdated {
        project: Project,
    },
    ProjectDeleted {
        project_id: String,
    },

    // Status updates
    ProjectStatusChanged {
        project_id: String,
        status: String,
    },

    // AI session updates
    AiSessionCreated {
        session: AiSession,
    },
    AiSessionStatusChanged {
        session_id: String,
        status: String,
    },
    AiSessionCompleted {
        session_id: String,
    },
    AiSessionFailed {
        session_id: String,
    },

    // Streaming output chunks for sessions (stdout/stderr)
    AiSessionOutputChunk {
        session_id: String,
        stream: String, // "stdout" | "stderr"
        content: String,
        #[serde(default)]
        seq: u64,
    },

    // Terminal session messages
    TerminalSessionStarted {
        session_id: String,
    },
    TerminalSessionEnded {
        session_id: String,
        exit_code: Option<i32>,
    },

    // Terminal output (binary data as base64)
    TerminalOutput {
        session_id: String,
        data: String, // base64 encoded binary data
    },

    // Terminal control messages (JSON)
    TerminalControl {
        session_id: String,
        message: TerminalControlMessage,
    },

    // Error handling
    Error {
        message: String,
    },

    // Ping/Pong for connection keep-alive
    Ping,
    Pong,

    // LLM Agent messages
    LlmAgentChunk {
        session_id: String,
        content: String,
    },
}

/// WebSocket connection actor
pub struct WebSocketConnection {
    /// Client ID for this connection
    client_id: String,
    /// Last heartbeat time
    hb: Instant,
    /// WebSocket server reference
    server: Addr<WebSocketServer>,
}

impl WebSocketConnection {
    pub fn new(server: Addr<WebSocketServer>) -> Self {
        Self {
            client_id: Uuid::new_v4().to_string(),
            hb: Instant::now(),
            server,
        }
    }

    /// Send heartbeat ping to client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(Duration::from_secs(30), |act, ctx| {
            // Check if client has sent pong back within 10 seconds
            if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
                // Client hasn't responded, disconnect
                tracing::warn!(
                    "WebSocket client {} failed heartbeat, disconnecting",
                    act.client_id
                );
                ctx.stop();
                return;
            }

            // Send ping
            ctx.ping(b"");
        });
    }
}

impl Actor for WebSocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("WebSocket connection started: {}", self.client_id);

        // Start heartbeat
        self.hb(ctx);

        // Register this connection with the server
        self.server.do_send(Connect {
            client_id: self.client_id.clone(),
            addr: ctx.address(),
        });

        // Send connection confirmation
        let msg = WebSocketMessage::Connected {
            client_id: self.client_id.clone(),
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        tracing::info!("WebSocket connection stopping: {}", self.client_id);

        // Unregister from server
        self.server.do_send(Disconnect {
            client_id: self.client_id.clone(),
        });

        Running::Stop
    }
}

/// Handle incoming WebSocket messages
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                let text_str = text.to_string();
                tracing::debug!("WebSocket message received: {}", text_str);

                // Parse incoming message
                match serde_json::from_str::<WebSocketMessage>(&text_str) {
                    Ok(WebSocketMessage::Ping) => {
                        let response = WebSocketMessage::Pong;
                        if let Ok(json) = serde_json::to_string(&response) {
                            ctx.text(json);
                        }
                    }
                    Ok(msg) => {
                        tracing::debug!("Received WebSocket message: {:?}", msg);
                        // Handle other message types as needed
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse WebSocket message: {}", e);
                        let error_msg = WebSocketMessage::Error {
                            message: format!("Invalid message format: {e}"),
                        };
                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            ctx.text(json);
                        }
                    }
                }
            }
            Ok(ws::Message::Binary(_)) => {
                tracing::debug!("Binary message received (ignored)");
            }
            Ok(ws::Message::Close(reason)) => {
                tracing::info!("WebSocket connection closed: {:?}", reason);
                ctx.close(reason);
                ctx.stop();
            }
            _ => {
                ctx.stop();
            }
        }
    }
}

/// WebSocket server that manages all connections
#[derive(Debug, Default)]
pub struct WebSocketServer {
    /// Active connections
    connections: HashMap<String, Addr<WebSocketConnection>>,
}

impl Actor for WebSocketServer {
    type Context = Context<Self>;
}

/// Message to connect a new WebSocket client
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub client_id: String,
    pub addr: Addr<WebSocketConnection>,
}

/// Message to connect a terminal WebSocket client
#[derive(Message)]
#[rtype(result = "()")]
pub struct TerminalConnect {
    pub client_id: String,
    pub session_id: String,
    #[allow(dead_code)]
    pub addr: Addr<TerminalWebSocketConnection>,
}

/// Message to disconnect a WebSocket client
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub client_id: String,
}

/// Message to broadcast to all connected clients
#[derive(Message)]
#[rtype(result = "()")]
pub struct Broadcast {
    pub message: WebSocketMessage,
}

/// Message to send to specific client
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendToClient {
    pub client_id: String,
    pub message: WebSocketMessage,
}

impl Handler<Connect> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Self::Context) {
        tracing::info!("WebSocket client connected: {}", msg.client_id);
        self.connections.insert(msg.client_id.clone(), msg.addr);
    }
}

impl Handler<TerminalConnect> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: TerminalConnect, _: &mut Self::Context) {
        tracing::info!(
            "Terminal WebSocket client connected: {} for session {}",
            msg.client_id,
            msg.session_id
        );
        // For now, we'll treat terminal connections the same as regular connections
        // TODO: Add separate tracking for terminal connections if needed
        // self.connections.insert(msg.client_id.clone(), msg.addr);
    }
}

impl Handler<Disconnect> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Self::Context) {
        tracing::info!("WebSocket client disconnected: {}", msg.client_id);
        self.connections.remove(&msg.client_id);
    }
}

impl Handler<Broadcast> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: Broadcast, _: &mut Self::Context) {
        tracing::debug!("Broadcasting message to {} clients", self.connections.len());

        // Send to all connected clients
        let mut to_remove = Vec::new();
        for (client_id, addr) in &self.connections {
            if addr.try_send(msg.message.clone()).is_err() {
                tracing::warn!("Failed to send message to client {}", client_id);
                to_remove.push(client_id.clone());
            }
        }

        // Remove failed connections
        for client_id in to_remove {
            self.connections.remove(&client_id);
        }
    }
}

impl Handler<SendToClient> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: SendToClient, _: &mut Self::Context) {
        if let Some(addr) = self.connections.get(&msg.client_id) {
            if addr.try_send(msg.message).is_err() {
                tracing::warn!("Failed to send message to client {}", msg.client_id);
                self.connections.remove(&msg.client_id);
            }
        } else {
            tracing::warn!("Client {} not found for direct message", msg.client_id);
        }
    }
}

impl Handler<WebSocketMessage> for WebSocketConnection {
    type Result = ();

    fn handle(&mut self, msg: WebSocketMessage, ctx: &mut Self::Context) {
        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }
}

/// WebSocket endpoint handler
pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<WebSocketServer>>,
) -> Result<HttpResponse, Error> {
    tracing::debug!("WebSocket connection request received");

    let resp = ws::start(
        WebSocketConnection::new(srv.get_ref().clone()),
        &req,
        stream,
    );

    tracing::debug!("WebSocket connection established");
    resp
}

/// AI session WebSocket endpoint handler
pub async fn ai_session_websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<WebSocketServer>>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let session_id = session_id.into_inner();
    tracing::debug!(
        "AI session WebSocket connection request received for session: {}",
        session_id
    );

    // For now, we'll use the same WebSocket connection logic
    // All clients will receive all AI session events
    let resp = ws::start(
        WebSocketConnection::new(srv.get_ref().clone()),
        &req,
        stream,
    );

    tracing::debug!("AI session WebSocket connection established");
    resp
}

/// Terminal WebSocket connection actor for PTY sessions
pub struct TerminalWebSocketConnection {
    /// Session ID for this terminal
    session_id: String,
    /// Client ID for this connection
    client_id: String,
    /// Last heartbeat time
    hb: Instant,
    /// WebSocket server reference
    server: Addr<WebSocketServer>,
}

impl TerminalWebSocketConnection {
    pub fn new(session_id: String, server: Addr<WebSocketServer>) -> Self {
        Self {
            session_id,
            client_id: Uuid::new_v4().to_string(),
            hb: Instant::now(),
            server,
        }
    }

    /// Send heartbeat ping to client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(Duration::from_secs(30), |act, ctx| {
            // Check if client has sent pong back within 10 seconds
            if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
                // Client hasn't responded, disconnect
                tracing::warn!(
                    "Terminal WebSocket client {} failed heartbeat, disconnecting",
                    act.client_id
                );
                ctx.stop();
                return;
            }

            // Send ping
            ctx.ping(b"");
        });
    }
}

impl Actor for TerminalWebSocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!(
            "Terminal WebSocket connection started: {} for session {}",
            self.client_id,
            self.session_id
        );

        // Start heartbeat
        self.hb(ctx);

        // Register this connection with the server
        self.server.do_send(TerminalConnect {
            client_id: self.client_id.clone(),
            session_id: self.session_id.clone(),
            addr: ctx.address(),
        });

        // Send connection confirmation
        let msg = WebSocketMessage::Connected {
            client_id: self.client_id.clone(),
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        tracing::info!("Terminal WebSocket connection stopping: {}", self.client_id);

        // Unregister from server
        self.server.do_send(Disconnect {
            client_id: self.client_id.clone(),
        });

        Running::Stop
    }
}

/// Handle incoming terminal WebSocket messages (both text and binary)
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for TerminalWebSocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                let text_str = text.to_string();
                tracing::debug!("Terminal WebSocket control message received: {}", text_str);

                // Parse incoming terminal control message
                match serde_json::from_str::<TerminalControlMessage>(&text_str) {
                    Ok(control_msg) => {
                        tracing::debug!("Received terminal control message: {:?}", control_msg);

                        // Handle terminal control message
                        // This should be forwarded to the terminal runner
                        // For now, we'll broadcast it back (echo)
                        let broadcast_msg = WebSocketMessage::TerminalControl {
                            session_id: self.session_id.clone(),
                            message: control_msg,
                        };

                        self.server.do_send(Broadcast {
                            message: broadcast_msg,
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse terminal control message: {}", e);
                        let error_msg = WebSocketMessage::Error {
                            message: format!("Invalid control message format: {e}"),
                        };
                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            ctx.text(json);
                        }
                    }
                }
            }
            Ok(ws::Message::Binary(data)) => {
                tracing::debug!(
                    "Terminal WebSocket binary data received: {} bytes",
                    data.len()
                );
                // Binary data should be sent to the terminal as input
                // For now, we'll echo it back as terminal output
                let base64_data = base64::engine::general_purpose::STANDARD.encode(&data);
                let output_msg = WebSocketMessage::TerminalOutput {
                    session_id: self.session_id.clone(),
                    data: base64_data,
                };

                self.server.do_send(Broadcast {
                    message: output_msg,
                });
            }
            Ok(ws::Message::Close(reason)) => {
                tracing::info!("Terminal WebSocket connection closed: {:?}", reason);
                ctx.close(reason);
                ctx.stop();
            }
            _ => {
                ctx.stop();
            }
        }
    }
}

impl Handler<WebSocketMessage> for TerminalWebSocketConnection {
    type Result = ();

    fn handle(&mut self, msg: WebSocketMessage, ctx: &mut Self::Context) {
        match msg {
            WebSocketMessage::TerminalOutput { session_id, data } => {
                // Only handle output for our session
                if session_id == self.session_id {
                    // Send binary data to client
                    if let Ok(binary_data) = base64::engine::general_purpose::STANDARD.decode(&data)
                    {
                        ctx.binary(binary_data);
                    }
                }
            }
            _ => {
                // Send other messages as JSON text
                if let Ok(json) = serde_json::to_string(&msg) {
                    ctx.text(json);
                }
            }
        }
    }
}

/// Terminal WebSocket endpoint handler for PTY sessions
pub async fn terminal_websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<WebSocketServer>>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let session_id = session_id.into_inner();
    tracing::debug!(
        "Terminal WebSocket connection request received for session: {}",
        session_id
    );

    let resp = ws::start(
        TerminalWebSocketConnection::new(session_id, srv.get_ref().clone()),
        &req,
        stream,
    );

    tracing::debug!("Terminal WebSocket connection established");
    resp
}

/// Utility functions for broadcasting messages
pub struct WebSocketBroadcaster {
    server: Arc<Addr<WebSocketServer>>,
}

#[allow(dead_code)]
impl WebSocketBroadcaster {
    pub fn new(server: Addr<WebSocketServer>) -> Self {
        Self {
            server: Arc::new(server),
        }
    }

    /// Broadcast project creation
    pub fn broadcast_project_created(&self, project: Project) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::ProjectCreated { project },
        });
    }

    /// Broadcast project update
    pub fn broadcast_project_updated(&self, project: Project) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::ProjectUpdated { project },
        });
    }

    /// Broadcast project deletion
    pub fn broadcast_project_deleted(&self, project_id: String) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::ProjectDeleted { project_id },
        });
    }

    /// Broadcast project status change
    pub fn broadcast_project_status_change(&self, project_id: String, status: String) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::ProjectStatusChanged { project_id, status },
        });
    }

    /// Broadcast AI session creation
    pub fn broadcast_ai_session_created(&self, session: AiSession) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::AiSessionCreated { session },
        });
    }

    /// Broadcast AI session status change
    pub fn broadcast_ai_session_status_change(&self, session_id: String, status: String) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::AiSessionStatusChanged { session_id, status },
        });
    }

    /// Broadcast AI session completion
    pub fn broadcast_ai_session_completed(&self, session_id: String) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::AiSessionCompleted { session_id },
        });
    }

    /// Broadcast AI session failure
    pub fn broadcast_ai_session_failed(&self, session_id: String) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::AiSessionFailed { session_id },
        });
    }

    /// Broadcast a single output chunk for a session
    pub fn broadcast_ai_output_chunk(
        &self,
        session_id: String,
        stream: &str,
        content: &str,
        seq: u64,
    ) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::AiSessionOutputChunk {
                session_id,
                stream: stream.to_string(),
                content: content.to_string(),
                seq,
            },
        });
    }

    /// Broadcast terminal session started
    pub async fn broadcast_terminal_session_started(&self, session_id: String) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::TerminalSessionStarted { session_id },
        });
    }

    /// Broadcast terminal session ended
    pub async fn broadcast_terminal_session_ended(
        &self,
        session_id: String,
        exit_code: Option<i32>,
    ) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::TerminalSessionEnded {
                session_id,
                exit_code,
            },
        });
    }

    /// Broadcast terminal output as binary data (base64 encoded)
    pub async fn broadcast_terminal_output(&self, session_id: String, data: Vec<u8>) {
        let base64_data = base64::engine::general_purpose::STANDARD.encode(data);
        self.server.do_send(Broadcast {
            message: WebSocketMessage::TerminalOutput {
                session_id,
                data: base64_data,
            },
        });
    }

    /// Broadcast terminal control message
    pub async fn broadcast_terminal_control_message(
        &self,
        session_id: String,
        message: TerminalControlMessage,
    ) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::TerminalControl {
                session_id,
                message,
            },
        });
    }

    /// Broadcast LLM agent chunk
    pub async fn broadcast_llm_agent_chunk(&self, session_id: String, content: String) {
        self.server.do_send(Broadcast {
            message: WebSocketMessage::LlmAgentChunk {
                session_id,
                content,
            },
        });
    }
}
