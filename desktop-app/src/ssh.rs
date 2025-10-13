use russh::client;
use std::net::TcpListener;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener as TokioTcpListener, TcpStream};

pub struct SshTunnel {
    pub local_port: u16,
    pub server: String,
    pub remote_port: u16,
    session: Arc<client::Handle<Client>>,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
}

#[derive(Debug)]
struct Client;

impl client::Handler for Client {
    type Error = russh::Error;
}

impl SshTunnel {
    pub async fn connect(
        server: &str,
        username: &str,
        key_path: Option<&str>,
    ) -> Result<Self, SshError> {
        tracing::info!("Attempting SSH connection to {}@{}", username, server);

        // Find an available local port
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let local_port = listener.local_addr()?.port();
        drop(listener); // Free the port

        let config = russh::client::Config {
            ..Default::default()
        };

        let client = Client;

        let mut session = russh::client::connect(Arc::new(config), (server, 22), client)
            .await
            .map_err(|e| {
                SshError::ConnectionFailed(format!("Failed to connect to {}: {}", server, e))
            })?;

        // Authenticate
        let key_paths = if let Some(key_path) = key_path {
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

        let mut auth_success = false;
        for key_path in &key_paths {
            if std::path::Path::new(key_path).exists() {
                tracing::info!("Trying SSH key: {}", key_path);
                match russh_keys::load_secret_key(key_path, None) {
                    Ok(key_pair) => {
                        match session
                            .authenticate_publickey(username, Arc::new(key_pair))
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "SSH authentication successful with key: {}",
                                    key_path
                                );
                                auth_success = true;
                                break;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Authentication failed with key {}: {}",
                                    key_path,
                                    e
                                );
                                continue;
                            }
                        }
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

        if !auth_success {
            return Err(SshError::AuthenticationFailed(
                "No valid SSH keys found or authentication failed".to_string(),
            ));
        }

        tracing::info!(
            "SSH connection established to {}@{}, setting up port forwarding",
            username,
            server
        );

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        // Start local TCP listener for port forwarding
        let listener = TokioTcpListener::bind(format!("127.0.0.1:{}", local_port))
            .await
            .map_err(SshError::Io)?;

        let session_arc = Arc::new(session);
        let session_clone = Arc::clone(&session_arc);
        let remote_port = 8081u32;

        // Spawn task to handle incoming connections
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                tracing::debug!("Accepted local connection from {}", addr);
                                let session = Arc::clone(&session_clone);
                                tokio::spawn(async move {
                                    if let Err(e) = handle_tunnel_connection(stream, session, remote_port).await {
                                        tracing::error!("Error handling tunnel connection: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Shutting down port forwarding listener");
                        break;
                    }
                }
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
            server: server.to_string(),
            remote_port: 8081,
            session: session_arc,
            shutdown_tx,
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub async fn disconnect(&mut self) -> Result<(), SshError> {
        tracing::info!("Disconnecting SSH tunnel");

        // Send shutdown signal to port forwarding listener
        if let Err(e) = self.shutdown_tx.send(()).await {
            tracing::warn!("Failed to send shutdown signal: {}", e);
        }

        // Disconnect SSH session
        self.session
            .disconnect(russh::Disconnect::ByApplication, "", "")
            .await?;

        tracing::info!("SSH tunnel disconnected");
        Ok(())
    }
}

/// Handle a single tunnel connection by forwarding data between local and remote
async fn handle_tunnel_connection(
    mut local_stream: TcpStream,
    session: Arc<client::Handle<Client>>,
    remote_port: u32,
) -> Result<(), SshError> {
    // Open direct-tcpip channel for port forwarding
    let channel = session
        .channel_open_direct_tcpip("localhost", remote_port, "localhost", 0)
        .await?;

    tracing::debug!("Opened SSH channel for port forwarding");

    // Split streams for bidirectional forwarding
    let (mut local_read, mut local_write) = local_stream.split();
    let mut ssh_channel = channel;

    // Forward data bidirectionally
    let mut buf = vec![0u8; 8192];
    loop {
        tokio::select! {
            // Local to SSH
            result = local_read.read(&mut buf) => {
                match result {
                    Ok(0) => {
                        tracing::debug!("Local connection closed");
                        break;
                    }
                    Ok(n) => {
                        if let Err(e) = ssh_channel.data(&buf[..n]).await {
                            tracing::error!("Failed to write to SSH channel: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from local stream: {}", e);
                        break;
                    }
                }
            }
            // SSH to Local
            msg = ssh_channel.wait() => {
                match msg {
                    Some(russh::ChannelMsg::Data { ref data }) => {
                        if let Err(e) = local_write.write_all(data).await {
                            tracing::error!("Failed to write to local stream: {}", e);
                            break;
                        }
                    }
                    Some(russh::ChannelMsg::Eof) => {
                        tracing::debug!("SSH channel EOF");
                        break;
                    }
                    Some(russh::ChannelMsg::Close) => {
                        tracing::debug!("SSH channel closed");
                        break;
                    }
                    None => {
                        tracing::debug!("SSH channel stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    tracing::debug!("Tunnel connection closed");
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Key error: {0}")]
    KeyError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),
    #[error("SSH keys error: {0}")]
    SshKeys(#[from] russh_keys::Error),
}
