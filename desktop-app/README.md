# nocodo Desktop App

A native desktop application for nocodo built with Rust and egui, providing SSH tunnel connectivity to remote manager servers.

## Features

- Native desktop UI using egui
- SSH connection with automatic key discovery
- Secure port forwarding to remote manager API
- Real-time project list display
- Connection status monitoring
- CLI test mode for headless operation

## Architecture

```
Desktop App (egui) on Laptop
    ↓ HTTP Request
localhost:RANDOM_PORT
    ↓ SSH Tunnel (russh)
dev-server:localhost:8081 (Manager API)
    ↓ SQLite Query
~/.local/share/nocodo/manager.db (server-side)
```

## Configuration

Create `~/.config/nocodo/desktop.toml`:

```toml
[ssh]
server = "dev-server.example.com"
username = "your-username"
ssh_key_path = "~/.ssh/id_ed25519"
remote_port = 8081
```

## Usage

### GUI Mode (Default)

```bash
cargo run --manifest-path desktop-app/Cargo.toml
```

### CLI Test Mode

Run the same code as the GUI but in headless mode with logging:

```bash
cargo run --manifest-path desktop-app/Cargo.toml -- --test
```

This will:
1. Load configuration
2. Establish SSH connection
3. Set up port forwarding
4. Test API connectivity
5. List all projects

## Building

```bash
# Development build
cargo build --manifest-path desktop-app/Cargo.toml

# Release build
cargo build --release --manifest-path desktop-app/Cargo.toml
```

## SSH Key Discovery

The app automatically searches for SSH keys in the following locations:
- `~/.ssh/id_rsa`
- `~/.ssh/id_ed25519`
- `~/.ssh/id_ecdsa`

Or you can specify a custom path in the configuration.

## Development

```bash
# Check for errors
cargo check --manifest-path desktop-app/Cargo.toml

# Run tests
cargo test --manifest-path desktop-app/Cargo.toml

# Format code
cargo fmt --manifest-path desktop-app/Cargo.toml

# Run linter
cargo clippy --manifest-path desktop-app/Cargo.toml
```

## Components

- **SSH Module** (`src/ssh.rs`): Handles SSH connection and port forwarding
- **API Client** (`src/api_client.rs`): HTTP client for manager API
- **Config** (`src/config.rs`): Configuration management
- **App** (`src/app.rs`): Main egui application
- **UI** (`src/ui/`): UI components (projects view)

## Dependencies

- `egui` / `eframe`: GUI framework
- `russh` / `russh-keys`: SSH client
- `reqwest`: HTTP client
- `tokio`: Async runtime
- `manager-models`: Shared data models with manager
