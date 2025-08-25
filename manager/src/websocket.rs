use crate::models::{AiSession, Project};
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
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
    Connected { client_id: String },
    Disconnected { client_id: String },

    // Project updates
    ProjectCreated { project: Project },
    ProjectUpdated { project: Project },
    ProjectDeleted { project_id: String },

    // Status updates
    ProjectStatusChanged { project_id: String, status: String },

    // AI session updates
    AiSessionCreated { session: AiSession },
    AiSessionStatusChanged { session_id: String, status: String },
    AiSessionCompleted { session_id: String },
    AiSessionFailed { session_id: String },

    // Streaming output chunks for sessions (stdout/stderr)
    AiSessionOutputChunk {
        session_id: String,
        stream: String, // "stdout" | "stderr"
        content: String,
        #[serde(default)]
        seq: u64,
    },

    // Error handling
    Error { message: String },

    // Ping/Pong for connection keep-alive
    Ping,
    Pong,
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
}
