use ssh2::Session;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct SshTunnel {
    pub local_port: u16,
    pub server: String,
    pub remote_port: u16,
    shutdown: Arc<AtomicBool>,
    _thread_handle: Option<thread::JoinHandle<()>>,
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

        // Establish SSH connection (blocking, so we do it in a spawned task)
        let server_owned = server.to_string();
        let username_owned = username.to_string();
        let key_path_owned = key_path.map(|s| s.to_string());

        let (tx, rx) = std::sync::mpsc::channel();

        // Spawn blocking connection task
        tokio::task::spawn_blocking(move || {
            let result =
                establish_ssh_connection(&server_owned, &username_owned, key_path_owned.as_deref());
            tx.send(result).ok();
        });

        let session = rx.recv().map_err(|e| {
            SshError::ConnectionFailed(format!("Failed to receive connection result: {}", e))
        })??;

        tracing::info!(
            "SSH connection established to {}@{}, setting up port forwarding",
            username,
            server
        );

        // Set up port forwarding in a background thread
        let session = Arc::new(std::sync::Mutex::new(session));
        let session_clone = Arc::clone(&session);
        let server_clone = server.to_string();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);

        let thread_handle = thread::spawn(move || {
            if let Err(e) = run_port_forward_loop(local_port, session_clone, 8081, shutdown_clone) {
                tracing::error!("Port forwarding error: {}", e);
            }
        });

        tracing::info!(
            "Port forwarding active: localhost:{} -> {}:{}",
            local_port,
            server,
            8081
        );

        Ok(Self {
            local_port,
            server: server_clone,
            remote_port: 8081,
            shutdown,
            _thread_handle: Some(thread_handle),
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub async fn disconnect(&mut self) -> Result<(), SshError> {
        tracing::info!("Disconnecting SSH tunnel");
        self.shutdown.store(true, Ordering::Relaxed);
        tracing::info!("SSH tunnel disconnected");
        Ok(())
    }
}

fn establish_ssh_connection(
    server: &str,
    username: &str,
    key_path: Option<&str>,
) -> Result<Session, SshError> {
    // Connect to SSH server
    let tcp = TcpStream::connect(format!("{}:22", server)).map_err(|e| {
        SshError::ConnectionFailed(format!("Failed to connect to {}: {}", server, e))
    })?;

    let mut session =
        Session::new().map_err(|e| SshError::Ssh(format!("Failed to create session: {}", e)))?;

    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| SshError::ConnectionFailed(format!("SSH handshake failed: {}", e)))?;

    // Try to authenticate with SSH keys
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

    let mut auth_success = false;
    for key_path in &key_paths {
        if Path::new(key_path).exists() {
            tracing::info!("Trying SSH key: {}", key_path);
            match session.userauth_pubkey_file(username, None, Path::new(key_path), None) {
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
        } else {
            tracing::debug!("Key file does not exist: {}", key_path);
        }
    }

    if !auth_success {
        return Err(SshError::AuthenticationFailed(
            "No valid SSH keys found or authentication failed".to_string(),
        ));
    }

    Ok(session)
}

fn run_port_forward_loop(
    local_port: u16,
    session: Arc<std::sync::Mutex<Session>>,
    remote_port: u16,
    shutdown: Arc<AtomicBool>,
) -> Result<(), SshError> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", local_port))?;
    listener.set_nonblocking(true)?;

    tracing::info!("Listening for connections on localhost:{}", local_port);

    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((local_stream, addr)) => {
                tracing::debug!("Accepted connection from {}", addr);
                let session_clone = Arc::clone(&session);
                thread::spawn(move || {
                    if let Err(e) =
                        handle_tunnel_connection(local_stream, session_clone, remote_port)
                    {
                        tracing::error!("Tunnel connection error: {}", e);
                    }
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No connections available, sleep briefly
                thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                tracing::error!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_tunnel_connection(
    mut local_stream: TcpStream,
    session: Arc<std::sync::Mutex<Session>>,
    remote_port: u16,
) -> Result<(), SshError> {
    let mut channel = session
        .lock()
        .unwrap()
        .channel_direct_tcpip("localhost", remote_port, None)
        .map_err(|e| SshError::Ssh(format!("Failed to open channel: {}", e)))?;

    tracing::debug!("Opened SSH channel for port forwarding");

    // Forward data bidirectionally
    let mut buf = vec![0u8; 8192];
    loop {
        // Try reading from local
        match local_stream.read(&mut buf) {
            Ok(0) => {
                tracing::debug!("Local connection closed");
                break;
            }
            Ok(n) => {
                if let Err(e) = channel.write_all(&buf[..n]) {
                    tracing::error!("Failed to write to SSH channel: {}", e);
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available, try reading from channel
            }
            Err(e) => {
                tracing::error!("Error reading from local stream: {}", e);
                break;
            }
        }

        // Try reading from SSH channel
        match channel.read(&mut buf) {
            Ok(0) => {
                tracing::debug!("SSH channel closed");
                break;
            }
            Ok(n) => {
                if let Err(e) = local_stream.write_all(&buf[..n]) {
                    tracing::error!("Failed to write to local stream: {}", e);
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available
            }
            Err(e) => {
                tracing::error!("Error reading from SSH channel: {}", e);
                break;
            }
        }

        // Brief sleep to avoid busy loop
        thread::sleep(std::time::Duration::from_millis(1));
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
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SSH error: {0}")]
    Ssh(String),
}
