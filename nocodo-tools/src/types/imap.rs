use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImapReaderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Optional path to config file. If not provided, uses agent settings."
    )]
    pub config_path: Option<String>,

    #[schemars(description = "The IMAP operation to execute")]
    pub operation: ImapOperation,

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
        #[schemars(
            description = "Mailbox pattern (e.g., '*' for all, 'INBOX/*' for INBOX subfolders)"
        )]
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
    pub success: bool,

    pub operation_type: String,

    pub data: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Default for SearchCriteria {
    fn default() -> Self {
        Self {
            from: None,
            to: None,
            subject: None,
            since_date: None,
            before_date: None,
            unseen_only: false,
            raw_query: None,
        }
    }
}
