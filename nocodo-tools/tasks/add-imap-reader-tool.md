# Add IMAP Reader Tool to nocodo-tools

**Status**: 📋 Not Started
**Priority**: Medium
**Created**: 2026-01-20

## Summary

Add a read-only IMAP email query tool (`imap_reader`) to nocodo-tools that enables AI agents to safely connect to IMAP mailboxes, search emails, fetch headers/metadata, and selectively download email content. This tool will follow a two-phase approach optimized for LLM decision-making: first fetch metadata, then selectively download full emails based on agent analysis.

## Problem Statement

AI agents need secure access to email mailboxes for:
- Email triage and classification
- Automated response generation
- Information extraction from emails
- Email-based workflow automation
- Customer support automation

Without a dedicated IMAP tool:
- **No standardized email access**: Each project would implement its own IMAP client
- **Inefficient bandwidth usage**: Downloading full emails when only metadata is needed
- **Security concerns**: Managing credentials and ensuring read-only access
- **No LLM optimization**: Generic IMAP clients aren't designed for agent-driven workflows

## Goals

1. **Create reusable imap_reader tool**: Single implementation in nocodo-tools
2. **Two-phase fetch optimization**: Fetch headers first, then selectively download full emails
3. **Username/password authentication**: Simple auth to start (OAuth2 deferred to v2)
4. **Cross-compilation support**: Use `rustls` instead of `native-tls` for portability
5. **Read-only operations**: No email deletion or flag modification in v1
6. **Schema introspection**: List mailboxes and query email metadata
7. **Secure credential storage**: Integration with agent settings system

## Architecture Overview

### Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Authentication** | Username/password (v1), OAuth2 (v2) | Simple auth covers most use cases; OAuth2 adds complexity |
| **TLS library** | `rustls` (not `native-tls`) | Cross-compilation support, pure Rust |
| **Message identifiers** | UIDs (not sequence numbers) | UIDs are stable across sessions; sequence numbers change |
| **Fetch strategy** | Two-phase (metadata → selective download) | Optimizes bandwidth; LLM decides what to download |
| **Connection lifecycle** | Per-request connection | Stateless, simpler implementation |
| **Write operations** | Read-only in v1 | Analysis tool, not mailbox manager |
| **Credential storage** | Agent settings (not in tool requests) | Security best practice |
| **Library choice** | `rust-imap` + `rustls-connector` | Mature, well-tested, async-capable |

### Tool Interface

```rust
// Request - supports multiple operation modes
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImapReaderRequest {
    /// Path to config file with credentials (optional, falls back to agent settings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,

    /// Operation to perform
    pub operation: ImapOperation,

    /// Optional timeout in seconds (default: 30s)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

// Operation modes
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ImapOperation {
    // Mailbox discovery
    #[serde(rename = "list_mailboxes")]
    ListMailboxes {
        #[serde(skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,  // e.g., "*" or "INBOX/*"
    },

    #[serde(rename = "mailbox_status")]
    MailboxStatus {
        mailbox: String,
    },

    // Search & metadata fetch (Phase 1 - LLM analysis)
    #[serde(rename = "search")]
    Search {
        mailbox: String,
        criteria: SearchCriteria,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<usize>,
    },

    #[serde(rename = "fetch_headers")]
    FetchHeaders {
        mailbox: String,
        message_uids: Vec<u32>,
    },

    // Full email fetch (Phase 2 - after LLM decides)
    #[serde(rename = "fetch_email")]
    FetchEmail {
        mailbox: String,
        message_uid: u32,
        #[serde(default)]
        include_html: bool,
        #[serde(default = "default_true")]
        include_text: bool,
    },
}

// Search criteria
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchCriteria {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since_date: Option<String>,  // RFC3501 format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_date: Option<String>,
    #[serde(default)]
    pub unseen_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_query: Option<String>,  // Fallback for advanced IMAP queries
}

// Response
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImapReaderResponse {
    pub success: bool,
    pub operation_type: String,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
```

### IMAP Connection Configuration

Credentials stored in agent settings (not in tool requests):

```rust
impl Agent for ImapAgent {
    fn static_settings_schema() -> Option<AgentSettingsSchema> {
        Some(AgentSettingsSchema {
            agent_name: "IMAP Email Agent".to_string(),
            section_name: "imap".to_string(),
            settings: vec![
                SettingDefinition {
                    name: "host".to_string(),
                    label: "IMAP Server".to_string(),
                    description: "IMAP server hostname (e.g., imap.gmail.com)".to_string(),
                    setting_type: SettingType::Text,
                    required: true,
                    default_value: None,
                },
                SettingDefinition {
                    name: "port".to_string(),
                    label: "Port".to_string(),
                    description: "IMAP port (default: 993 for TLS)".to_string(),
                    setting_type: SettingType::Number,
                    required: false,
                    default_value: Some("993".to_string()),
                },
                SettingDefinition {
                    name: "username".to_string(),
                    label: "Email Address".to_string(),
                    description: "Your email address for IMAP login".to_string(),
                    setting_type: SettingType::Text,
                    required: true,
                    default_value: None,
                },
                SettingDefinition {
                    name: "password".to_string(),
                    label: "Password".to_string(),
                    description: "IMAP password or app-specific password".to_string(),
                    setting_type: SettingType::Password,
                    required: true,
                    default_value: None,
                },
            ],
        })
    }
}
```

## Implementation Plan

### Phase 1: Core IMAP Client and Connection

#### 1.1 Create IMAP Module Structure

Create new module in nocodo-tools:
```
nocodo-tools/
  src/
    imap/
      mod.rs              # Public interface and main executor
      client.rs           # IMAP client wrapper with rustls
      operations.rs       # Operation implementations
      types.rs            # Internal data structures
      formatter.rs        # Result formatting for LLMs
    types/
      imap.rs             # ImapReaderRequest/Response types
```

#### 1.2 Implement IMAP Client with rustls

**File**: `nocodo-tools/src/imap/client.rs`

```rust
use anyhow::{Context, Result};
use imap::Client;
use std::time::Duration;

pub struct ImapClient {
    session: imap::Session<Box<dyn std::io::Read + std::io::Write + Send>>,
}

impl ImapClient {
    /// Create new IMAP client with rustls TLS
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        timeout: Duration,
    ) -> Result<Self> {
        // Build TLS client with rustls
        let client = imap::ClientBuilder::new(host, port)
            .connect()
            .context("Failed to connect to IMAP server")?;

        // Authenticate
        let session = client
            .login(username, password)
            .map_err(|e| anyhow::anyhow!("IMAP login failed: {}", e.0))?;

        Ok(Self { session })
    }

    /// Select mailbox and return session
    pub fn select_mailbox(&mut self, mailbox: &str) -> Result<()> {
        self.session
            .select(mailbox)
            .context(format!("Failed to select mailbox: {}", mailbox))?;
        Ok(())
    }

    /// Examine mailbox (read-only)
    pub fn examine_mailbox(&mut self, mailbox: &str) -> Result<()> {
        self.session
            .examine(mailbox)
            .context(format!("Failed to examine mailbox: {}", mailbox))?;
        Ok(())
    }

    /// Get reference to session for operations
    pub fn session(&mut self) -> &mut imap::Session<Box<dyn std::io::Read + std::io::Write + Send>> {
        &mut self.session
    }

    /// Logout and close connection
    pub fn logout(mut self) -> Result<()> {
        self.session
            .logout()
            .context("Failed to logout")?;
        Ok(())
    }
}
```

#### 1.3 Implement Core Operations

**File**: `nocodo-tools/src/imap/operations.rs`

```rust
use super::client::ImapClient;
use super::types::*;
use anyhow::{Context, Result};
use std::collections::HashSet;

/// List mailboxes
pub fn list_mailboxes(
    client: &mut ImapClient,
    pattern: Option<&str>,
) -> Result<Vec<MailboxInfo>> {
    let pattern = pattern.unwrap_or("*");
    let mailboxes = client
        .session()
        .list(Some(""), Some(pattern))
        .context("Failed to list mailboxes")?;

    let result = mailboxes
        .iter()
        .map(|mb| MailboxInfo {
            name: mb.name().to_string(),
            delimiter: mb.delimiter().map(|c| c.to_string()),
            flags: mb.attributes().iter().map(|a| format!("{:?}", a)).collect(),
        })
        .collect();

    Ok(result)
}

/// Get mailbox status
pub fn mailbox_status(
    client: &mut ImapClient,
    mailbox: &str,
) -> Result<MailboxStatusInfo> {
    let status = client
        .session()
        .status(mailbox, "(MESSAGES RECENT UNSEEN UIDNEXT UIDVALIDITY)")
        .context("Failed to get mailbox status")?;

    Ok(MailboxStatusInfo {
        mailbox: mailbox.to_string(),
        messages: status.messages,
        recent: status.recent,
        unseen: status.unseen,
        uid_next: status.uid_next,
        uid_validity: status.uid_validity,
    })
}

/// Search emails
pub fn search_emails(
    client: &mut ImapClient,
    mailbox: &str,
    criteria: &SearchCriteria,
    limit: Option<usize>,
) -> Result<Vec<u32>> {
    // Select mailbox
    client.examine_mailbox(mailbox)?;

    // Build IMAP search query
    let query = build_search_query(criteria)?;

    // Execute search
    let uids = client
        .session()
        .uid_search(&query)
        .context("Failed to execute search")?;

    // Convert HashSet to Vec and apply limit
    let mut uid_vec: Vec<u32> = uids.into_iter().collect();
    uid_vec.sort_unstable_by(|a, b| b.cmp(a)); // Sort descending (newest first)

    if let Some(limit) = limit {
        uid_vec.truncate(limit);
    }

    Ok(uid_vec)
}

/// Fetch email headers/metadata
pub fn fetch_headers(
    client: &mut ImapClient,
    mailbox: &str,
    uids: &[u32],
) -> Result<Vec<EmailHeader>> {
    if uids.is_empty() {
        return Ok(Vec::new());
    }

    // Select mailbox
    client.examine_mailbox(mailbox)?;

    // Build UID set
    let uid_set = build_uid_set(uids);

    // Fetch envelope, flags, and internal date
    let messages = client
        .session()
        .uid_fetch(&uid_set, "(UID ENVELOPE FLAGS INTERNALDATE RFC822.SIZE)")
        .context("Failed to fetch headers")?;

    let mut headers = Vec::new();
    for msg in messages.iter() {
        if let Some(envelope) = msg.envelope() {
            headers.push(EmailHeader {
                uid: msg.uid.unwrap_or(0),
                subject: envelope
                    .subject
                    .as_ref()
                    .and_then(|s| String::from_utf8(s.to_vec()).ok()),
                from: envelope
                    .from
                    .as_ref()
                    .map(|addrs| format_addresses(addrs)),
                to: envelope
                    .to
                    .as_ref()
                    .map(|addrs| format_addresses(addrs)),
                date: envelope
                    .date
                    .as_ref()
                    .and_then(|d| String::from_utf8(d.to_vec()).ok()),
                flags: msg
                    .flags()
                    .iter()
                    .map(|f| format!("{:?}", f))
                    .collect(),
                size: msg.size,
            });
        }
    }

    Ok(headers)
}

/// Fetch full email
pub fn fetch_email(
    client: &mut ImapClient,
    mailbox: &str,
    uid: u32,
    include_html: bool,
    include_text: bool,
) -> Result<EmailContent> {
    // Select mailbox
    client.examine_mailbox(mailbox)?;

    // Fetch full message
    let messages = client
        .session()
        .uid_fetch(uid.to_string(), "RFC822")
        .context("Failed to fetch email")?;

    let message = messages
        .iter()
        .next()
        .context("Email not found")?;

    let body = message.body().context("Email has no body")?;

    // Parse MIME structure
    let parsed = parse_email_body(body, include_html, include_text)?;

    Ok(parsed)
}

// Helper functions

fn build_search_query(criteria: &SearchCriteria) -> Result<String> {
    if let Some(raw) = &criteria.raw_query {
        return Ok(raw.clone());
    }

    let mut parts = Vec::new();

    if let Some(from) = &criteria.from {
        parts.push(format!("FROM \"{}\"", escape_query_string(from)));
    }
    if let Some(to) = &criteria.to {
        parts.push(format!("TO \"{}\"", escape_query_string(to)));
    }
    if let Some(subject) = &criteria.subject {
        parts.push(format!("SUBJECT \"{}\"", escape_query_string(subject)));
    }
    if let Some(since) = &criteria.since_date {
        parts.push(format!("SINCE {}", since));
    }
    if let Some(before) = &criteria.before_date {
        parts.push(format!("BEFORE {}", before));
    }
    if criteria.unseen_only {
        parts.push("UNSEEN".to_string());
    }

    if parts.is_empty() {
        Ok("ALL".to_string())
    } else {
        Ok(parts.join(" "))
    }
}

fn escape_query_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn build_uid_set(uids: &[u32]) -> String {
    uids.iter()
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn format_addresses(addrs: &[imap::types::Address]) -> Vec<String> {
    addrs
        .iter()
        .filter_map(|addr| {
            let name = addr.name.as_ref().and_then(|n| String::from_utf8(n.to_vec()).ok());
            let mailbox = addr.mailbox.as_ref().and_then(|m| String::from_utf8(m.to_vec()).ok());
            let host = addr.host.as_ref().and_then(|h| String::from_utf8(h.to_vec()).ok());

            match (mailbox, host) {
                (Some(m), Some(h)) => {
                    if let Some(n) = name {
                        Some(format!("{} <{}@{}>", n, m, h))
                    } else {
                        Some(format!("{}@{}", m, h))
                    }
                }
                _ => None,
            }
        })
        .collect()
}

fn parse_email_body(
    body: &[u8],
    include_html: bool,
    include_text: bool,
) -> Result<EmailContent> {
    // TODO: Implement proper MIME parsing
    // For v1, return raw body as text
    let body_text = String::from_utf8_lossy(body).to_string();

    Ok(EmailContent {
        text_body: if include_text { Some(body_text.clone()) } else { None },
        html_body: if include_html { None } else { None }, // TODO: Parse HTML parts
        attachments: Vec::new(), // TODO: Parse attachments
    })
}
```

#### 1.4 Define Internal Types

**File**: `nocodo-tools/src/imap/types.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxInfo {
    pub name: String,
    pub delimiter: Option<String>,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxStatusInfo {
    pub mailbox: String,
    pub messages: Option<u32>,
    pub recent: Option<u32>,
    pub unseen: Option<u32>,
    pub uid_next: Option<u32>,
    pub uid_validity: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailHeader {
    pub uid: u32,
    pub subject: Option<String>,
    pub from: Option<Vec<String>>,
    pub to: Option<Vec<String>>,
    pub date: Option<String>,
    pub flags: Vec<String>,
    pub size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailContent {
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub filename: Option<String>,
    pub content_type: String,
    pub size: usize,
}
```

### Phase 2: Integrate with nocodo-tools Type System

#### 2.1 Create Type Definitions

**File**: `nocodo-tools/src/types/imap.rs`

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Execute IMAP operations to read emails from mailboxes. This is a READ-ONLY tool.
/// It does NOT support sending emails, deleting emails, or modifying flags.
///
/// Typical workflow:
/// 1. list_mailboxes - Discover available mailboxes
/// 2. search - Find emails matching criteria
/// 3. fetch_headers - Get metadata for matching emails (efficient)
/// 4. fetch_email - Download full email content (only for selected emails)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImapReaderRequest {
    /// Path to config file with IMAP credentials (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Optional path to config file. If not provided, uses agent settings.")]
    pub config_path: Option<String>,

    /// Operation to perform
    #[schemars(description = "The IMAP operation to execute")]
    pub operation: ImapOperation,

    /// Timeout in seconds (default: 30)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Operation timeout in seconds. Defaults to 30.")]
    pub timeout_seconds: Option<u64>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ImapOperation {
    #[serde(rename = "list_mailboxes")]
    ListMailboxes {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Mailbox pattern (e.g., '*' for all, 'INBOX/*' for INBOX subfolders)")]
        pattern: Option<String>,
    },

    #[serde(rename = "mailbox_status")]
    MailboxStatus {
        #[schemars(description = "Mailbox name (e.g., 'INBOX')")]
        mailbox: String,
    },

    #[serde(rename = "search")]
    Search {
        #[schemars(description = "Mailbox to search (e.g., 'INBOX')")]
        mailbox: String,
        #[schemars(description = "Search criteria")]
        criteria: SearchCriteria,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Maximum number of UIDs to return")]
        limit: Option<usize>,
    },

    #[serde(rename = "fetch_headers")]
    FetchHeaders {
        #[schemars(description = "Mailbox name")]
        mailbox: String,
        #[schemars(description = "List of message UIDs to fetch")]
        message_uids: Vec<u32>,
    },

    #[serde(rename = "fetch_email")]
    FetchEmail {
        #[schemars(description = "Mailbox name")]
        mailbox: String,
        #[schemars(description = "Message UID to fetch")]
        message_uid: u32,
        #[serde(default)]
        #[schemars(description = "Include HTML body if available")]
        include_html: bool,
        #[serde(default = "default_true")]
        #[schemars(description = "Include text body (default: true)")]
        include_text: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchCriteria {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Filter by sender email/name")]
    pub from: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Filter by recipient email/name")]
    pub to: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Filter by subject text")]
    pub subject: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Emails on or after this date (RFC3501 format: DD-MMM-YYYY)")]
    pub since_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Emails before this date (RFC3501 format: DD-MMM-YYYY)")]
    pub before_date: Option<String>,

    #[serde(default)]
    #[schemars(description = "Only return unseen (unread) emails")]
    pub unseen_only: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Raw IMAP search query (advanced users only)")]
    pub raw_query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImapReaderResponse {
    /// Whether the operation succeeded
    pub success: bool,

    /// Type of operation performed
    pub operation_type: String,

    /// Operation result data (structure varies by operation)
    pub data: serde_json::Value,

    /// Optional message (errors or warnings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
```

#### 2.2 Update ToolRequest and ToolResponse Enums

**File**: `nocodo-tools/src/types/core.rs`

Add new variant to `ToolRequest`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ToolRequest {
    // ... existing variants
    #[serde(rename = "imap_reader")]
    ImapReader(super::imap::ImapReaderRequest),
}
```

Add new variant to `ToolResponse`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResponse {
    // ... existing variants
    #[serde(rename = "imap_reader")]
    ImapReader(super::imap::ImapReaderResponse),
}
```

#### 2.3 Update Type Module Exports

**File**: `nocodo-tools/src/types/mod.rs`

Add:
```rust
pub mod imap;
// ... existing modules

pub use imap::{ImapReaderRequest, ImapReaderResponse, ImapOperation, SearchCriteria};
```

### Phase 3: Implement Main Executor

#### 3.1 Create Main IMAP Module

**File**: `nocodo-tools/src/imap/mod.rs`

```rust
use crate::tool_error::ToolError;
use crate::types::{ImapReaderRequest, ImapReaderResponse, ImapOperation, ToolResponse};
use anyhow::{Context, Result};
use std::time::Duration;

mod client;
mod operations;
mod types;

use client::ImapClient;

/// Execute an imap_reader tool request
pub async fn execute_imap_reader(
    request: ImapReaderRequest,
) -> Result<ToolResponse, ToolError> {
    // Load IMAP credentials
    let config = load_imap_config(request.config_path.as_deref())?;

    // Set timeout
    let timeout = Duration::from_secs(request.timeout_seconds.unwrap_or(30));

    // Connect to IMAP server
    let mut client = ImapClient::connect(
        &config.host,
        config.port,
        &config.username,
        &config.password,
        timeout,
    )
    .map_err(|e| ToolError::ExecutionError(format!("Failed to connect to IMAP: {}", e)))?;

    // Execute operation
    let (operation_type, data) = match request.operation {
        ImapOperation::ListMailboxes { pattern } => {
            let mailboxes = operations::list_mailboxes(&mut client, pattern.as_deref())
                .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
            ("list_mailboxes".to_string(), serde_json::to_value(mailboxes)?)
        }
        ImapOperation::MailboxStatus { mailbox } => {
            let status = operations::mailbox_status(&mut client, &mailbox)
                .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
            ("mailbox_status".to_string(), serde_json::to_value(status)?)
        }
        ImapOperation::Search {
            mailbox,
            criteria,
            limit,
        } => {
            let uids = operations::search_emails(&mut client, &mailbox, &criteria, limit)
                .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
            ("search".to_string(), serde_json::to_value(uids)?)
        }
        ImapOperation::FetchHeaders {
            mailbox,
            message_uids,
        } => {
            let headers = operations::fetch_headers(&mut client, &mailbox, &message_uids)
                .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
            ("fetch_headers".to_string(), serde_json::to_value(headers)?)
        }
        ImapOperation::FetchEmail {
            mailbox,
            message_uid,
            include_html,
            include_text,
        } => {
            let email = operations::fetch_email(
                &mut client,
                &mailbox,
                message_uid,
                include_html,
                include_text,
            )
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
            ("fetch_email".to_string(), serde_json::to_value(email)?)
        }
    };

    // Logout
    let _ = client.logout();

    let response = ImapReaderResponse {
        success: true,
        operation_type,
        data,
        message: None,
    };

    Ok(ToolResponse::ImapReader(response))
}

#[derive(Debug, Clone)]
struct ImapConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
}

fn load_imap_config(config_path: Option<&str>) -> Result<ImapConfig, ToolError> {
    // TODO: Implement config file loading
    // TODO: Integrate with agent settings
    // For now, return error - agent must provide config
    Err(ToolError::InvalidInput(
        "IMAP config loading not yet implemented. Use agent settings.".to_string(),
    ))
}
```

#### 3.2 Update ToolExecutor

**File**: `nocodo-tools/src/tool_executor.rs`

Add import:
```rust
use crate::imap;
```

Add match arm in `execute()`:
```rust
pub async fn execute(&self, request: ToolRequest) -> Result<ToolResponse> {
    match request {
        // ... existing match arms
        ToolRequest::ImapReader(req) => {
            imap::execute_imap_reader(req).await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }
}
```

### Phase 4: Add Dependencies

#### 4.1 Update Cargo.toml

**File**: `nocodo-tools/Cargo.toml`

Add dependencies:
```toml
[dependencies]
# ... existing dependencies
imap = { version = "3.0.0-alpha.15", default-features = false, features = ["rustls-tls"] }
rustls-connector = "0.19.0"
mail-parser = "0.9"  # For MIME parsing (Phase 2)
```

### Phase 5: Testing

#### 5.1 Unit Tests

**File**: `nocodo-tools/src/imap/operations.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_query() {
        let criteria = SearchCriteria {
            from: Some("sender@example.com".to_string()),
            to: None,
            subject: Some("Meeting".to_string()),
            since_date: None,
            before_date: None,
            unseen_only: true,
            raw_query: None,
        };

        let query = build_search_query(&criteria).unwrap();
        assert!(query.contains("FROM"));
        assert!(query.contains("SUBJECT"));
        assert!(query.contains("UNSEEN"));
    }

    #[test]
    fn test_build_uid_set() {
        let uids = vec![1, 3, 5, 7];
        let uid_set = build_uid_set(&uids);
        assert_eq!(uid_set, "1,3,5,7");
    }

    #[test]
    fn test_escape_query_string() {
        let input = r#"test "quoted" text"#;
        let escaped = escape_query_string(input);
        assert!(escaped.contains(r#"\""#));
    }
}
```

#### 5.2 Integration Tests (Manual)

Since integration tests require a real IMAP server, document manual test procedures:

**Manual Test Checklist**:
1. Test against Gmail (with app-specific password)
2. Test against a local test IMAP server (Greenmail or similar)
3. Test various search criteria combinations
4. Test error handling (invalid credentials, network timeout)
5. Test large mailbox handling (pagination/limits)

### Phase 6: Documentation

#### 6.1 Update nocodo-tools README

**File**: `nocodo-tools/README.md`

Add section:
```markdown
### IMAP Reader Tool

Read-only IMAP email query tool for AI agents.

**Features:**
- List mailboxes and get status information
- Search emails with flexible criteria
- Two-phase fetch: metadata first, selective full download
- Username/password authentication with TLS (rustls)
- Cross-compilation support
- Optimized for LLM-driven email workflows

**Usage Example:**
```rust
use nocodo_tools::{ToolExecutor, ToolRequest, ImapReaderRequest, ImapOperation};

let executor = ToolExecutor::new(base_path);

// Search for unread emails from specific sender
let request = ToolRequest::ImapReader(ImapReaderRequest {
    config_path: None, // Uses agent settings
    operation: ImapOperation::Search {
        mailbox: "INBOX".to_string(),
        criteria: SearchCriteria {
            from: Some("important@example.com".to_string()),
            unseen_only: true,
            ..Default::default()
        },
        limit: Some(10),
    },
    timeout_seconds: Some(30),
});

let response = executor.execute(request).await?;
```

**Typical Workflow:**
1. `list_mailboxes` - Discover available mailboxes
2. `search` - Find emails matching criteria (returns UIDs)
3. `fetch_headers` - Get metadata for filtered emails
4. LLM analyzes headers and decides which emails to download
5. `fetch_email` - Download selected full emails

**Security:**
- Read-only operations (no email deletion or modification)
- TLS encryption with rustls
- Credentials stored securely in agent settings
- Timeout enforcement to prevent hanging
```

## Files Changed

### New Files
- `nocodo-tools/src/imap/mod.rs` - Main module and executor
- `nocodo-tools/src/imap/client.rs` - IMAP client with rustls
- `nocodo-tools/src/imap/operations.rs` - Operation implementations
- `nocodo-tools/src/imap/types.rs` - Internal data structures
- `nocodo-tools/src/types/imap.rs` - Request/Response types
- `nocodo-tools/tasks/add-imap-reader-tool.md` - This task document

### Modified Files
- `nocodo-tools/Cargo.toml` - Add imap and rustls dependencies
- `nocodo-tools/src/lib.rs` - Add imap module export
- `nocodo-tools/src/types/mod.rs` - Add imap types export
- `nocodo-tools/src/types/core.rs` - Add ImapReader variants
- `nocodo-tools/src/tool_executor.rs` - Add imap_reader execution
- `nocodo-tools/README.md` - Document new tool

## Testing & Validation

### Unit Tests
```bash
cd nocodo-tools
cargo test imap
```

### Integration Tests (Manual)
Requires IMAP server access. See Phase 5.2 for checklist.

### Full Build & Quality Checks
```bash
cd nocodo-tools
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

### Cross-Compilation Test
```bash
# Test that rustls-based build works for cross-compilation
cargo build --target x86_64-unknown-linux-musl
```

## Success Criteria

- [ ] imap_reader tool integrated into nocodo-tools
- [ ] All unit tests pass
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Documentation complete
- [ ] `list_mailboxes` operation working
- [ ] `search` operation with various criteria working
- [ ] `fetch_headers` returns correct metadata
- [ ] `fetch_email` downloads full email content
- [ ] Error handling for connection failures
- [ ] Timeout enforcement working
- [ ] rustls TLS connection successful
- [ ] Ready for use in nocodo-agents

## Future Enhancements (v2)

### Phase 2 Features (Deferred)
1. **OAuth2 Authentication**
   - Gmail OAuth2 support
   - Microsoft 365 OAuth2 support
   - Token refresh handling

2. **Advanced MIME Parsing**
   - Proper HTML/text body extraction
   - Attachment download support
   - Multipart message handling

3. **Write Operations** (Carefully considered)
   - Mark as read/unread
   - Move emails between mailboxes
   - Flag operations

4. **Performance Optimizations**
   - Connection pooling
   - Batch fetch operations
   - Local email cache (SQLite)

5. **IDLE Support**
   - Real-time email notifications
   - Mailbox monitoring

## References

- **rust-imap source**: `~/Projects/rust-imap/`
- **rust-imap README**: `~/Projects/rust-imap/README.md`
- **rust-imap examples**: `~/Projects/rust-imap/examples/`
- **rustls example**: `~/Projects/rust-imap/examples/rustls.rs`
- **IMAP RFC 3501**: https://tools.ietf.org/html/rfc3501
- **rust-imap docs**: https://docs.rs/imap/
- **rustls-connector docs**: https://docs.rs/rustls-connector/

## Notes

- This is a pure addition - no breaking changes to existing tools
- The tool is designed for email reading/analysis, not mailbox management
- Two-phase fetch pattern is critical for efficient LLM workflows
- rustls chosen over native-tls for cross-compilation support
- OAuth2 support deferred to v2 to keep initial implementation simple
- Connection per request is simpler than connection pooling for v1
- MIME parsing will be basic in v1, enhanced in v2
- Agent credential storage integration is key for security
