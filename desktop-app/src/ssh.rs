use russh::client;
use std::net::TcpListener;
use std::sync::Arc;

#[derive(Debug)]
pub struct SshTunnel {
    pub local_port: u16,
    pub server: String,
    pub remote_port: u16,
    #[allow(dead_code)]
    handle: Option<Box<dyn std::any::Any + Send>>, // Placeholder for connection handle
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

        let mut session = russh::client::connect(Arc::new(config), (server, 22), client).await
            .map_err(|e| SshError::ConnectionFailed(format!("Failed to connect to {}: {}", server, e)))?;

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
                        match session.authenticate_publickey(username, Arc::new(key_pair)).await {
                            Ok(_) => {
                                tracing::info!("SSH authentication successful with key: {}", key_path);
                                auth_success = true;
                                break;
                            }
                            Err(e) => {
                                tracing::warn!("Authentication failed with key {}: {}", key_path, e);
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
                "No valid SSH keys found or authentication failed".to_string()
            ));
        }

        // For now, we'll just establish the connection without port forwarding
        // TODO: Implement proper port forwarding
        tracing::info!("SSH connection established to {}@{} (port forwarding not yet implemented)", username, server);

        Ok(Self {
            local_port,
            server: server.to_string(),
            remote_port: 8081,
            handle: None, // TODO: Store session handle
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub async fn disconnect(&mut self) -> Result<(), SshError> {
        // TODO: Implement proper disconnect
        tracing::info!("SSH tunnel disconnect requested (not yet implemented)");
        Ok(())
    }
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