# Bootstrap App Specification

## Overview

The Bootstrap app is the client-side entry point for nocodo, responsible for managing cloud provider credentials, orchestrating server creation, and providing secure communication between the user's local environment and remote Operator servers. Written in Rust using Actix Web framework with SQLite for local data persistence.

## Architecture

### Components

1. **Authentication Service** - Handles nocodo.com authentication
2. **Cloud Provider Manager** - Abstracts cloud provider APIs
3. **Server Orchestrator** - Manages Operator server lifecycle
4. **Security Manager** - Handles encryption and key management
5. **API Service** - Provides REST endpoints for web interface
6. **Database Layer** - SQLite with encrypted storage

### Technology Stack

- **Language**: Rust
- **Web Framework**: Actix Web
- **Database**: SQLite with SQLCipher for encryption
- **Cloud SDKs**: Scaleway SDK, with planned support for DigitalOcean, Vultr, Linode
- **Cryptography**: `ring` or `rustls` for encryption
- **Serialization**: `serde` with `ts-rs` for TypeScript type generation

## Core Features

### 1. Authentication Management

**Endpoint**: `POST /auth/login`
**Purpose**: Authenticate with nocodo.com and establish local session

```rust
#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: String,
    pub expires_at: DateTime<Utc>,
}
```

**Implementation Notes**:
- JWT tokens stored in encrypted local storage
- Token refresh mechanism for long-lived sessions
- Secure communication with nocodo.com auth service

### 2. Cloud Provider Management

**Endpoints**:
- `GET /providers` - List supported cloud providers
- `POST /providers/{provider}/keys` - Store API keys
- `GET /providers/{provider}/keys` - List stored keys (masked)
- `DELETE /providers/{provider}/keys/{key_id}` - Remove API keys

```rust
#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CloudProvider {
    pub name: String,
    pub display_name: String,
    pub required_credentials: Vec<CredentialField>,
    pub regions: Vec<Region>,
    pub instance_types: Vec<InstanceType>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CredentialField {
    pub name: String,
    pub display_name: String,
    pub field_type: CredentialFieldType,
    pub is_secret: bool,
    pub validation_regex: Option<String>,
}
```

**Implementation Notes**:
- Plugin architecture for cloud provider support
- Credential validation before storage
- Encrypted storage of sensitive credentials
- Rate limiting for API key operations

### 3. Server Orchestration

**Endpoints**:
- `POST /operator/create` - Create new Operator server
- `GET /operator/status` - Get server status
- `POST /operator/start` - Start stopped server
- `POST /operator/stop` - Stop running server
- `POST /operator/destroy` - Destroy server
- `GET /operator/images` - List available server images

```rust
#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OperatorServer {
    pub id: String,
    pub provider: String,
    pub region: String,
    pub instance_type: String,
    pub status: ServerStatus,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub image_id: Option<String>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ServerStatus {
    Creating,
    Running,
    Stopped,
    Error,
    Destroyed,
}
```

**Server Setup Process**:
1. Create Ubuntu 22.04 LTS instance (4-8GB RAM, 2-4 vCPU)
2. Configure SSH access with key-based authentication
3. Run hardening script (firewall, fail2ban, automatic updates)
4. Install Docker and Docker Compose
5. Install Manager daemon and dependencies
6. Create system services and configure startup
7. Generate and store server image for future reuse

### 4. Security Management

**Features**:
- AES-256 encryption for sensitive data at rest
- Separate encryption password (can match auth password)
- Secure key derivation using PBKDF2 or Argon2
- Certificate pinning for nocodo.com communication
- Regular security audits and updates

```rust
#[derive(Serialize, Deserialize)]
pub struct EncryptedStorage {
    pub salt: Vec<u8>,
    pub iv: Vec<u8>,
    pub encrypted_data: Vec<u8>,
    pub algorithm: String,
}
```

### 5. Image Management

**Features**:
- Server image creation after successful Manager installation
- Image reuse for faster server provisioning
- Version management for different Manager releases
- Automatic cleanup of old images

## Database Schema

```sql
-- Users table
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    auth_token TEXT,
    token_expires_at INTEGER,
    encryption_salt BLOB NOT NULL,
    created_at INTEGER NOT NULL
);

-- Cloud provider credentials
CREATE TABLE provider_credentials (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    credential_name TEXT NOT NULL,
    encrypted_data BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id)
);

-- Operator servers
CREATE TABLE operator_servers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    region TEXT NOT NULL,
    instance_id TEXT,
    instance_type TEXT NOT NULL,
    status TEXT NOT NULL,
    ip_address TEXT,
    image_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id)
);

-- Server images
CREATE TABLE server_images (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    region TEXT NOT NULL,
    image_id TEXT NOT NULL,
    manager_version TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

## Configuration

```toml
# config.toml
[server]
host = "127.0.0.1"
port = 8080
workers = 4

[database]
path = "./data/nocodo.db"
encryption_required = true

[auth]
nocodo_api_url = "https://api.nocodo.com"
token_refresh_threshold = 3600

[security]
encryption_algorithm = "AES-256-GCM"
key_derivation = "Argon2id"
key_derivation_iterations = 100000

[logging]
level = "info"
file = "./logs/bootstrap.log"
```

## API Specification

### Error Handling

```rust
#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}
```

### Common Response Format

```rust
#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub timestamp: DateTime<Utc>,
}
```

## Security Considerations

1. **Credential Storage**: All cloud provider credentials encrypted at rest
2. **Network Security**: TLS 1.3 for all external communications
3. **Access Control**: Local-only API access, no remote connections
4. **Audit Logging**: All sensitive operations logged
5. **Key Management**: Secure key derivation and storage
6. **Server Hardening**: Automatic security updates and firewall configuration

## Deployment and Development

### Build Requirements

```toml
[dependencies]
actix-web = "4.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls", "chrono"] }
ts-rs = "7.0"
ring = "0.16"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4"] }
```

### Development Setup

1. Install Rust toolchain
2. Set up development database: `sqlite3 dev.db < migrations/init.sql`
3. Configure development config: `cp config.example.toml config.toml`
4. Run with: `cargo run`

## Clarification Questions

1. **Multi-tenancy**: Should the Bootstrap app support multiple nocodo.com accounts on the same machine?
2. **Cloud Provider Priority**: Should we implement all cloud providers simultaneously or focus on Scaleway first?
3. **Backup Strategy**: How should we handle backup and recovery of local encrypted data?
4. **Update Mechanism**: How should the Bootstrap app handle self-updates?
5. **Resource Limits**: What are the minimum and maximum resource limits for Operator servers?
6. **Network Configuration**: Should we support custom VPC/networking configurations?
7. **Monitoring**: What level of monitoring/telemetry should be built into the Bootstrap app?
8. **Offline Mode**: Should any functionality work when disconnected from nocodo.com?

## Future Enhancements

- Mobile app version for iOS/Android
- Support for additional cloud providers (AWS, GCP, Azure)
- Advanced server configuration options
- Multi-region deployments
- Team/organization management
- Cost tracking and budgeting features
- Integration with CI/CD pipelines
