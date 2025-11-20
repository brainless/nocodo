use base64::{engine::general_purpose, Engine as _};
use russh::keys::{key::PrivateKeyWithHashAlg, load_secret_key, ssh_key, PublicKeyBase64};
use russh::*;
use sha2::{Digest, Sha256};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};
use tokio::sync::Mutex;

pub struct SshTunnel {
    local_port: u16,
    pub server: String,
    pub remote_port: u16,
    session: Arc<Mutex<Option<client::Handle<ClientHandler>>>>,
    shutdown: Arc<tokio::sync::Notify>,
    _task_handle: Option<tokio::task::JoinHandle<()>>,
}

// Simple client handler that accepts all server keys for development
struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // For development: accept all server keys
        // TODO: In production, verify against known_hosts
        tracing::debug!("Server key accepted (development mode)");
        Ok(true)
    }
}

impl SshTunnel {
    pub async fn connect(
        server: &str,
        username: &str,
        key_path: Option<&str>,
        port: u16,
        remote_port: u16,
    ) -> Result<Self, SshError> {
        tracing::info!("Attempting SSH connection to {}@{}", username, server);

        // Find an available local port
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let local_port = listener.local_addr()?.port();
        drop(listener); // Free the port

        // Load SSH key
        let key_paths: Vec<String> = if let Some(key_path) = key_path {
            vec![key_path.to_string()]
        } else {
            // Try default key locations
            let home = std::env::var("HOME").unwrap_or_default();
            vec![
                format!("{}/.ssh/id_rsa", home),
                format!("{}/.ssh/id_ed25519", home),
                format!("{}/.ssh/id_ecdsa", home),
            ]
        };

        let mut key_pair = None;
        for key_path in &key_paths {
            if Path::new(key_path).exists() {
                tracing::info!("Trying SSH key: {}", key_path);
                match load_secret_key(key_path, None) {
                    Ok(key) => {
                        tracing::info!("SSH key loaded successfully: {}", key_path);
                        key_pair = Some(key);
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load key {}: {}", key_path, e);
                        continue;
                    }
                }
            } else {
                tracing::debug!("Key file does not exist: {}", key_path);
            }
        }

        let key_pair = key_pair
            .ok_or_else(|| SshError::AuthenticationFailed("No valid SSH keys found".to_string()))?;

        // Create SSH client configuration with keepalive
        let config = client::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(600)), // 10 minutes (increased from 5)
            keepalive_interval: Some(std::time::Duration::from_secs(30)), // Send keepalive every 30 seconds
            keepalive_max: 3, // Allow 3 missed keepalives before closing connection
            ..Default::default()
        };

        let config = Arc::new(config);
        let handler = ClientHandler;

        // Connect to SSH server
        tracing::info!("Connecting to SSH server {}:{}", server, port);
        let mut session = client::connect(config, (server, port), handler)
            .await
            .map_err(|e| SshError::ConnectionFailed(format!("Connection failed: {}", e)))?;

        // Authenticate with public key
        tracing::info!("Authenticating with public key");
        let best_hash = session
            .best_supported_rsa_hash()
            .await
            .map_err(|e| SshError::AuthenticationFailed(format!("Failed to get RSA hash: {}", e)))?
            .flatten();

        let auth_result = session
            .authenticate_publickey(
                username,
                PrivateKeyWithHashAlg::new(Arc::new(key_pair), best_hash),
            )
            .await
            .map_err(|e| SshError::AuthenticationFailed(format!("Authentication failed: {}", e)))?;

        if !auth_result.success() {
            return Err(SshError::AuthenticationFailed(
                "Authentication rejected by server".to_string(),
            ));
        }

        tracing::info!(
            "SSH connection established to {}@{}, setting up port forwarding",
            username,
            server
        );

        let session = Arc::new(Mutex::new(Some(session)));
        let session_clone = Arc::clone(&session);
        let shutdown = Arc::new(tokio::sync::Notify::new());
        let shutdown_clone = Arc::clone(&shutdown);
        let server_clone = server.to_string();

        // Start port forwarding task
        let task_handle = tokio::spawn(async move {
            if let Err(e) =
                run_port_forward_loop(local_port, session_clone, remote_port, shutdown_clone).await
            {
                tracing::error!("Port forwarding error: {}", e);
            }
        });

        tracing::info!(
            "Port forwarding active: localhost:{} -> {}:{}",
            local_port,
            server,
            remote_port
        );

        Ok(Self {
            local_port,
            server: server_clone,
            remote_port,
            session,
            shutdown,
            _task_handle: Some(task_handle),
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub async fn disconnect(&mut self) -> Result<(), SshError> {
        tracing::info!("Disconnecting SSH tunnel");

        // Signal shutdown
        self.shutdown.notify_waiters();

        // Close session
        if let Some(session) = self.session.lock().await.take() {
            if let Err(e) = session
                .disconnect(Disconnect::ByApplication, "", "English")
                .await
            {
                tracing::warn!("Error during disconnect: {}", e);
            }
        }

        tracing::info!("SSH tunnel disconnected");
        Ok(())
    }
}

async fn run_port_forward_loop(
    local_port: u16,
    session: Arc<Mutex<Option<client::Handle<ClientHandler>>>>,
    remote_port: u16,
    shutdown: Arc<tokio::sync::Notify>,
) -> Result<(), SshError> {
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind(format!("127.0.0.1:{}", local_port).parse().unwrap())?;
    let listener = socket.listen(128)?;

    tracing::info!("Listening for connections on localhost:{}", local_port);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((local_stream, addr)) => {
                        tracing::debug!("Accepted connection from {}", addr);
                        let session_clone = Arc::clone(&session);
                        tokio::spawn(async move {
                            if let Err(e) = handle_tunnel_connection(local_stream, session_clone, remote_port).await {
                                tracing::error!("Tunnel connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Error accepting connection: {}", e);
                    }
                }
            }
            _ = shutdown.notified() => {
                tracing::info!("Port forwarding loop shutting down");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_tunnel_connection(
    mut local_stream: TcpStream,
    session: Arc<Mutex<Option<client::Handle<ClientHandler>>>>,
    remote_port: u16,
) -> Result<(), SshError> {
    let session_guard = session.lock().await;
    let session_handle = session_guard
        .as_ref()
        .ok_or_else(|| SshError::Ssh("Session closed".to_string()))?;

    let mut channel = session_handle
        .channel_open_direct_tcpip("localhost", remote_port as u32, "localhost", 0)
        .await
        .map_err(|e| SshError::Ssh(format!("Failed to open channel: {}", e)))?;

    drop(session_guard); // Release the lock

    tracing::debug!("Opened SSH channel for port forwarding");

    // Forward data bidirectionally
    let mut stream_closed = false;
    let mut buf = vec![0u8; 8192];

    loop {
        tokio::select! {
            // Read from local stream, write to SSH channel
            result = local_stream.read(&mut buf), if !stream_closed => {
                match result {
                    Ok(0) => {
                        tracing::debug!("Local connection closed");
                        stream_closed = true;
                        if let Err(e) = channel.eof().await {
                            tracing::error!("Failed to send EOF to channel: {}", e);
                        }
                    }
                    Ok(n) => {
                        if let Err(e) = channel.data(&buf[..n]).await {
                            tracing::error!("Failed to send data to SSH channel: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from local stream: {}", e);
                        break;
                    }
                }
            }
            // Read from SSH channel, write to local stream
            Some(msg) = channel.wait() => {
                match msg {
                    ChannelMsg::Data { ref data } => {
                        if let Err(e) = local_stream.write_all(data).await {
                            tracing::error!("Failed to write to local stream: {}", e);
                            break;
                        }
                    }
                    ChannelMsg::Eof => {
                        tracing::debug!("SSH channel EOF received");
                        if !stream_closed {
                            if let Err(e) = channel.eof().await {
                                tracing::error!("Failed to send EOF to channel: {}", e);
                            }
                        }
                        break;
                    }
                    ChannelMsg::WindowAdjusted { .. } => {
                        // Ignore window adjustment messages
                    }
                    _ => {
                        tracing::debug!("Received other channel message: {:?}", msg);
                    }
                }
            }
        }
    }

    tracing::debug!("Tunnel connection closed");
    Ok(())
}

/// Get the default SSH key path that exists on the system
/// Returns the first existing SSH key path or the default id_rsa path if none exist
pub fn get_default_ssh_key_path() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();

    let ssh_dir = Path::new(&home).join(".ssh");

    let key_names = vec!["id_ed25519", "id_rsa", "id_ecdsa"];

    // Return the first key that exists
    for key_name in &key_names {
        let key_path = ssh_dir.join(key_name);
        if key_path.exists() {
            return key_path.to_string_lossy().to_string();
        }
    }

    // If no keys exist, return the default id_rsa path
    ssh_dir.join("id_rsa").to_string_lossy().to_string()
}

/// Read SSH public key from .pub file
/// Returns the full public key content
pub fn read_ssh_public_key(key_path: Option<&str>) -> Result<String, SshError> {
    // Find key path (same logic as connect)
    let key_paths: Vec<String> = if let Some(key_path) = key_path {
        vec![key_path.to_string()]
    } else {
        // Try default key locations - use HOME on Unix, USERPROFILE on Windows
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        vec![
            format!("{}/.ssh/id_rsa", home),
            format!("{}/.ssh/id_ed25519", home),
            format!("{}/.ssh/id_ecdsa", home),
        ]
    };

    // Try to find the corresponding .pub file
    for key_path in &key_paths {
        let pub_key_path = format!("{}.pub", key_path);
        if Path::new(&pub_key_path).exists() {
            match std::fs::read_to_string(&pub_key_path) {
                Ok(content) => {
                    // Trim whitespace and return
                    return Ok(content.trim().to_string());
                }
                Err(_) => continue,
            }
        }
    }

    Err(SshError::AuthenticationFailed(
        "No SSH public key found".to_string(),
    ))
}

/// Calculate SSH key fingerprint in SHA256 format
/// Returns a string like "SHA256:base64hash"
pub fn calculate_ssh_fingerprint(key_path: Option<&str>) -> Result<String, SshError> {
    // Find key path (same logic as connect)
    let key_paths: Vec<String> = if let Some(key_path) = key_path {
        vec![key_path.to_string()]
    } else {
        // Try default key locations
        let home = std::env::var("HOME").unwrap_or_default();
        vec![
            format!("{}/.ssh/id_rsa", home),
            format!("{}/.ssh/id_ed25519", home),
            format!("{}/.ssh/id_ecdsa", home),
        ]
    };

    // Load SSH key
    let mut key_pair = None;
    for key_path in &key_paths {
        if Path::new(key_path).exists() {
            match load_secret_key(key_path, None) {
                Ok(key) => {
                    key_pair = Some(key);
                    break;
                }
                Err(_) => continue,
            }
        }
    }

    let key_pair = key_pair
        .ok_or_else(|| SshError::AuthenticationFailed("No valid SSH keys found".to_string()))?;

    // Get public key bytes
    let public_key = key_pair.public_key();
    let public_key_bytes = public_key.public_key_bytes();

    // Calculate SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(public_key_bytes);
    let hash = hasher.finalize();

    // Encode to base64
    let base64_hash = general_purpose::STANDARD.encode(hash);

    // Format as "SHA256:base64hash"
    Ok(format!("SHA256:{}", base64_hash))
}

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SSH error: {0}")]
    Ssh(String),
}
