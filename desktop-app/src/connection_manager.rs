use crate::api_client::{ApiClient, ApiError};
use crate::ssh::{SshError, SshTunnel};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};

/// Connection type - either SSH tunnel or direct local connection
#[derive(Debug, Clone)]
pub enum ConnectionType {
    Ssh {
        server: String,
        username: String,
        key_path: Option<String>,
        port: u16,
        remote_port: u16,
    },
    Local {
        port: u16,
    },
}

/// Connection manager that handles SSH tunnels and API client lifecycle
pub struct ConnectionManager {
    connection_type: Arc<RwLock<Option<ConnectionType>>>,
    tunnel: Arc<Mutex<Option<SshTunnel>>>,
    api_client: Arc<RwLock<Option<ApiClient>>>,
    connected: Arc<RwLock<bool>>,
    keepalive_shutdown: Arc<tokio::sync::Notify>,
    health_check_shutdown: Arc<tokio::sync::Notify>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connection_type: Arc::new(RwLock::new(None)),
            tunnel: Arc::new(Mutex::new(None)),
            api_client: Arc::new(RwLock::new(None)),
            connected: Arc::new(RwLock::new(false)),
            keepalive_shutdown: Arc::new(tokio::sync::Notify::new()),
            health_check_shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Connect using SSH tunnel
    pub async fn connect_ssh(
        &self,
        server: &str,
        username: &str,
        key_path: Option<&str>,
        port: u16,
        remote_port: u16,
    ) -> Result<(), ConnectionError> {
        tracing::info!("Connecting to {}@{} via SSH", username, server);

        // Store connection details
        let conn_type = ConnectionType::Ssh {
            server: server.to_string(),
            username: username.to_string(),
            key_path: key_path.map(|s| s.to_string()),
            port,
            remote_port,
        };
        *self.connection_type.write().await = Some(conn_type.clone());

        // Establish SSH tunnel
        let tunnel = SshTunnel::connect(server, username, key_path, port, remote_port)
            .await
            .map_err(ConnectionError::SshError)?;

        let local_port = tunnel.local_port();
        tracing::info!("SSH tunnel established on port {}", local_port);

        // Create API client
        let api_client = ApiClient::new(format!("http://localhost:{}", local_port));

        // Store tunnel and client
        *self.tunnel.lock().await = Some(tunnel);
        *self.api_client.write().await = Some(api_client);
        *self.connected.write().await = true;

        // Start keepalive and health check tasks
        self.start_keepalive_task().await;
        self.start_health_check_task().await;

        Ok(())
    }

    /// Connect to local manager instance
    pub async fn connect_local(&self, port: u16) -> Result<(), ConnectionError> {
        tracing::info!("Connecting to local manager on port {}", port);

        let conn_type = ConnectionType::Local { port };
        *self.connection_type.write().await = Some(conn_type);

        // Create API client
        let api_client = ApiClient::new(format!("http://localhost:{}", port));

        // Test connection
        match api_client.get_settings().await {
            Ok(_) => {
                *self.api_client.write().await = Some(api_client);
                *self.connected.write().await = true;

                // Start health check task
                self.start_health_check_task().await;

                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to connect to local manager: {}", e);
                Err(ConnectionError::ApiError(e))
            }
        }
    }

    /// Disconnect and cleanup resources
    pub async fn disconnect(&self) {
        tracing::info!("Disconnecting...");

        // Signal shutdown to background tasks
        self.keepalive_shutdown.notify_waiters();
        self.health_check_shutdown.notify_waiters();

        // Disconnect tunnel if exists
        if let Some(mut tunnel) = self.tunnel.lock().await.take() {
            if let Err(e) = tunnel.disconnect().await {
                tracing::warn!("Error disconnecting tunnel: {}", e);
            }
        }

        // Clear state
        *self.api_client.write().await = None;
        *self.connected.write().await = false;
        *self.connection_type.write().await = None;

        tracing::info!("Disconnected successfully");
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Get the API client if connected
    pub async fn get_api_client(&self) -> Option<ApiClient> {
        self.api_client.read().await.clone()
    }

    /// Perform a health check on the connection
    pub async fn check_health(&self) -> bool {
        if !*self.connected.read().await {
            return false;
        }

        if let Some(client) = self.api_client.read().await.as_ref() {
            // Try a lightweight API call to verify connection
            match tokio::time::timeout(Duration::from_secs(5), client.get_settings()).await {
                Ok(Ok(_)) => true,
                Ok(Err(e)) => {
                    tracing::warn!("Health check failed: {}", e);
                    false
                }
                Err(_) => {
                    tracing::warn!("Health check timed out");
                    false
                }
            }
        } else {
            false
        }
    }

    /// Attempt to reconnect if connection is dead
    pub async fn reconnect(&self) -> Result<(), ConnectionError> {
        tracing::info!("Attempting to reconnect...");

        // Get connection type
        let conn_type = self.connection_type.read().await.clone();

        match conn_type {
            Some(ConnectionType::Ssh {
                server,
                username,
                key_path,
                port,
                remote_port,
            }) => {
                // Disconnect existing connection
                self.disconnect().await;

                // Wait a bit before reconnecting
                tokio::time::sleep(Duration::from_secs(1)).await;

                // Reconnect
                self.connect_ssh(&server, &username, key_path.as_deref(), port, remote_port)
                    .await
            }
            Some(ConnectionType::Local { port }) => {
                // Disconnect existing connection
                self.disconnect().await;

                // Wait a bit before reconnecting
                tokio::time::sleep(Duration::from_secs(1)).await;

                // Reconnect
                self.connect_local(port).await
            }
            None => Err(ConnectionError::NoConnectionInfo),
        }
    }

    /// Start keepalive task for SSH connections
    async fn start_keepalive_task(&self) {
        let shutdown = Arc::clone(&self.keepalive_shutdown);

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60)); // Send keepalive every 60 seconds

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // The SSH library should handle keepalive internally,
                        // but we can perform periodic health checks here
                        tracing::trace!("Keepalive tick");
                    }
                    _ = shutdown.notified() => {
                        tracing::info!("Keepalive task shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Start health check task that monitors connection and auto-reconnects
    async fn start_health_check_task(&self) {
        let connection_manager = ConnectionManagerHandle {
            connection_type: Arc::clone(&self.connection_type),
            tunnel: Arc::clone(&self.tunnel),
            api_client: Arc::clone(&self.api_client),
            connected: Arc::clone(&self.connected),
            keepalive_shutdown: Arc::clone(&self.keepalive_shutdown),
            health_check_shutdown: Arc::clone(&self.health_check_shutdown),
        };

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(30)); // Check every 30 seconds
            let mut consecutive_failures = 0;

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if *connection_manager.connected.read().await {
                            // Perform health check
                            if let Some(client) = connection_manager.api_client.read().await.as_ref() {
                                match tokio::time::timeout(
                                    Duration::from_secs(5),
                                    client.get_settings()
                                ).await {
                                    Ok(Ok(_)) => {
                                        tracing::trace!("Health check passed");
                                        consecutive_failures = 0;
                                    }
                                    Ok(Err(e)) => {
                                        consecutive_failures += 1;
                                        tracing::warn!(
                                            "Health check failed (attempt {}): {}",
                                            consecutive_failures,
                                            e
                                        );

                                        // Try to reconnect after 2 consecutive failures
                                        if consecutive_failures >= 2 {
                                            tracing::info!("Attempting auto-reconnect...");
                                            *connection_manager.connected.write().await = false;

                                            // Get connection details for reconnection
                                            let conn_type = connection_manager.connection_type.read().await.clone();

                                            if let Some(conn_type) = conn_type {
                                                match conn_type {
                                                    ConnectionType::Ssh {
                                                        server,
                                                        username,
                                                        key_path,
                                                        port,
                                                        remote_port,
                                                    } => {
                                                        // Close old tunnel
                                                        if let Some(mut tunnel) = connection_manager.tunnel.lock().await.take() {
                                                            let _ = tunnel.disconnect().await;
                                                        }

                                                        // Attempt reconnection
                                                        match SshTunnel::connect(
                                                            &server,
                                                            &username,
                                                            key_path.as_deref(),
                                                            port,
                                                            remote_port,
                                                        )
                                                        .await
                                                        {
                                                            Ok(tunnel) => {
                                                                let local_port = tunnel.local_port();
                                                                tracing::info!("Reconnected successfully on port {}", local_port);

                                                                let api_client = ApiClient::new(format!("http://localhost:{}", local_port));

                                                                *connection_manager.tunnel.lock().await = Some(tunnel);
                                                                *connection_manager.api_client.write().await = Some(api_client);
                                                                *connection_manager.connected.write().await = true;
                                                                consecutive_failures = 0;
                                                            }
                                                            Err(e) => {
                                                                tracing::error!("Reconnection failed: {}", e);
                                                            }
                                                        }
                                                    }
                                                    ConnectionType::Local { port } => {
                                                        // For local connections, just recreate the client
                                                        let api_client = ApiClient::new(format!("http://localhost:{}", port));

                                                        if api_client.get_settings().await.is_ok() {
                                                            *connection_manager.api_client.write().await = Some(api_client);
                                                            *connection_manager.connected.write().await = true;
                                                            consecutive_failures = 0;
                                                            tracing::info!("Reconnected to local manager");
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        consecutive_failures += 1;
                                        tracing::warn!(
                                            "Health check timed out (attempt {})",
                                            consecutive_failures
                                        );
                                    }
                                }
                            }
                        }
                    }
                    _ = connection_manager.health_check_shutdown.notified() => {
                        tracing::info!("Health check task shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Get the current local port (for SSH tunnels)
    pub async fn get_local_port(&self) -> Option<u16> {
        self.tunnel.lock().await.as_ref().map(|t| t.local_port())
    }

    /// Get connection information
    pub async fn get_connection_info(&self) -> Option<String> {
        match self.connection_type.read().await.as_ref()? {
            ConnectionType::Ssh { server, .. } => Some(server.clone()),
            ConnectionType::Local { .. } => Some("localhost".to_string()),
        }
    }
}

/// A handle to the connection manager for use in background tasks
#[derive(Clone)]
struct ConnectionManagerHandle {
    connection_type: Arc<RwLock<Option<ConnectionType>>>,
    tunnel: Arc<Mutex<Option<SshTunnel>>>,
    api_client: Arc<RwLock<Option<ApiClient>>>,
    connected: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    keepalive_shutdown: Arc<tokio::sync::Notify>,
    health_check_shutdown: Arc<tokio::sync::Notify>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("SSH error: {0}")]
    SshError(#[from] SshError),
    #[error("API error: {0}")]
    ApiError(#[from] ApiError),
    #[error("No connection information available")]
    NoConnectionInfo,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
