use crate::tool_error::ToolError;
use crate::types::{ImapOperation, ImapReaderRequest, ImapReaderResponse, ToolResponse};
use anyhow::Result;
use std::time::Duration;

mod client;
mod operations;
mod types;

use client::ImapClient;

pub async fn execute_imap_reader(request: ImapReaderRequest) -> Result<ToolResponse, ToolError> {
    let config = load_imap_config(request.config_path.as_deref())?;

    let timeout = Duration::from_secs(request.timeout_seconds.unwrap_or(30));

    let mut client = ImapClient::connect(
        &config.host,
        config.port,
        &config.username,
        &config.password,
        timeout,
    )
    .map_err(|e| ToolError::ExecutionError(format!("Failed to connect to IMAP: {}", e)))?;

    let (operation_type, data) = match request.operation {
        ImapOperation::ListMailboxes { pattern } => {
            let mailboxes = operations::list_mailboxes(&mut client, pattern.as_deref())
                .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
            (
                "list_mailboxes".to_string(),
                serde_json::to_value(mailboxes)?,
            )
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

    let _ = client.logout();

    let response = ImapReaderResponse {
        success: true,
        operation_type,
        data,
        message: None,
    };

    Ok(ToolResponse::ImapReader(response))
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ImapConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
}

fn load_imap_config(config_path: Option<&str>) -> Result<ImapConfig, ToolError> {
    if let Some(path) = config_path {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read config file: {}", e)))?;

        let config: ImapConfig = serde_json::from_str(&content)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    } else {
        Err(ToolError::InvalidInput(
            "IMAP config not provided. Please provide a config_path or configure IMAP settings."
                .to_string(),
        ))
    }
}
