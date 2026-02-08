use crate::{
    storage::AgentStorage,
    types::{
        Message as StorageMessage, MessageRole, Session, SessionStatus,
        ToolCall as StorageToolCall, ToolCallStatus,
    },
    Agent, AgentSettingsSchema, AgentTool, SettingDefinition, SettingType,
};
use anyhow::{self, Context};
use async_trait::async_trait;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall as LlmToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage, Role};
use nocodo_tools::types::ToolRequest;
use nocodo_tools::ToolExecutor;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;
use tempfile::NamedTempFile;

#[cfg(test)]
mod tests;

const IMAP_EMAIL_AGENT_SYSTEM_PROMPT: &str = r#"You are an email analysis expert specialized in IMAP email management and triage.

Your role is to help users manage their email inbox, find information, summarize
conversations, and automate email-based workflows. You have access to the imap_reader
tool which provides READ-ONLY access to IMAP mailboxes.

# Available IMAP Operations

1. **list_mailboxes** - Discover available mailboxes (INBOX, Sent, Drafts, folders)
2. **mailbox_status** - Get counts (total messages, unseen, recent) for a mailbox
3. **search** - Find emails matching criteria (from, to, subject, date, unseen)
4. **fetch_headers** - Get email metadata (subject, from, to, date, flags, size)
5. **fetch_email** - Download full email content (text/HTML body)

# Two-Phase Workflow (IMPORTANT)

Always follow this efficient workflow:

**Phase 1: Discovery & Filtering**
1. Use `search` to find relevant email UIDs based on criteria
2. Use `fetch_headers` to get metadata for those UIDs
3. Analyze headers (subjects, senders, dates, sizes) to understand content

**Phase 2: Selective Download**
4. Based on user needs and header analysis, decide which emails need full content
5. Use `fetch_email` ONLY for emails that require full body analysis
6. Avoid downloading large emails unless specifically requested

This approach minimizes bandwidth and keeps analysis focused.

# Best Practices

1. **Start broad, then narrow**: Use search to filter, headers to analyze, fetch for details
2. **Respect mailbox size**: Use limits in search queries for large mailboxes
3. **Analyze before downloading**: Headers contain 80% of useful information
4. **Batch operations**: Fetch multiple headers in one call when possible
5. **Explain decisions**: Tell users why you're fetching specific emails
6. **Handle errors gracefully**: Network issues, authentication failures, etc.

# Example Workflows

## Email Triage
User: "Show me unread emails from important-client@example.com"
1. search(mailbox="INBOX", criteria={from: "important-client@", unseen_only: true})
2. fetch_headers(uids=<results>)
3. Present summary with subjects, dates, and sizes
4. If user wants details, fetch_email for specific UIDs

## Information Extraction
User: "Find the order confirmation from Amazon last week"
1. search(mailbox="INBOX", criteria={from: "amazon", subject: "order", since: <7_days_ago>})
2. fetch_headers(uids=<results>)
3. Identify likely matches from subjects
4. fetch_email for the most relevant email(s)
5. Extract order information from email body

## Mailbox Exploration
User: "What folders do I have and what's in them?"
1. list_mailboxes()
2. For each interesting mailbox: mailbox_status()
3. Present overview of mailbox structure and counts

# Search Criteria Format

The search operation accepts these criteria:
- `from`: Filter by sender email/name
- `to`: Filter by recipient email/name
- `subject`: Filter by subject text
- `since_date`: Emails on/after date (RFC3501 format: DD-MMM-YYYY, e.g., "15-JAN-2026")
- `before_date`: Emails before date (RFC3501 format)
- `unseen_only`: Only unread emails (boolean)
- `raw_query`: Advanced IMAP search query for power users

# Date Format

IMAP dates use RFC3501 format: DD-MMM-YYYY
Examples: "01-JAN-2026", "15-DEC-2025", "30-JUN-2025"
Convert user's natural language dates ("last week", "yesterday") to this format.

# Security & Limitations

- **Read-only**: You CANNOT delete, move, or modify emails (v1 limitation)
- **No sending**: You CANNOT send emails via IMAP
- **Session-based**: Credentials are configured at session start
- **Timeout**: Long operations may timeout (typically 30 seconds)

# Error Handling

If operations fail:
- Authentication errors → Check credentials in settings
- Mailbox not found → Use list_mailboxes to see available mailboxes
- Network timeout → Retry with smaller limits or simpler queries
- Search returns too many results → Add more specific criteria or use limits

Always provide helpful context when errors occur so users can resolve issues.
"#;

pub struct ImapEmailAgent<S: AgentStorage> {
    client: Arc<dyn LlmClient>,
    storage: Arc<S>,
    tool_executor: Arc<ToolExecutor>,
    imap_config: ImapConfig,
    system_prompt: String,
}

struct ImapConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
}

impl<S: AgentStorage> ImapEmailAgent<S> {
    pub fn new(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        tool_executor: Arc<ToolExecutor>,
        host: String,
        port: u16,
        username: String,
        password: String,
    ) -> Self {
        Self {
            client,
            storage,
            tool_executor,
            imap_config: ImapConfig {
                host,
                port,
                username,
                password,
            },
            system_prompt: IMAP_EMAIL_AGENT_SYSTEM_PROMPT.to_string(),
        }
    }

    pub fn from_settings(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        tool_executor: Arc<ToolExecutor>,
        settings: &HashMap<String, String>,
    ) -> anyhow::Result<Self> {
        let host = settings
            .get("host")
            .context("Missing 'host' in IMAP settings")?
            .clone();

        let port = settings
            .get("port")
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(993);

        let username = settings
            .get("username")
            .context("Missing 'username' in IMAP settings")?
            .clone();

        let password = settings
            .get("password")
            .context("Missing 'password' in IMAP settings")?
            .clone();

        Ok(Self::new(
            client,
            storage,
            tool_executor,
            host,
            port,
            username,
            password,
        ))
    }

    async fn get_session(&self, session_id: i64) -> anyhow::Result<Session> {
        self.storage
            .get_session(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))
    }

    fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
        self.tools()
            .into_iter()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }

    async fn build_messages(&self, session_id: i64) -> anyhow::Result<Vec<LlmMessage>> {
        let db_messages = self.storage.get_messages(session_id).await?;

        db_messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::User => Role::User,
                    MessageRole::Assistant => Role::Assistant,
                    MessageRole::System => Role::System,
                    MessageRole::Tool => Role::User,
                };

                Ok(LlmMessage {
                    role,
                    content: vec![ContentBlock::Text { text: msg.content }],
                })
            })
            .collect()
    }

    async fn execute_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call: &LlmToolCall,
    ) -> anyhow::Result<()> {
        let mut tool_request =
            AgentTool::parse_tool_call(tool_call.name(), tool_call.arguments().clone())?;

        // Handle credential injection for IMAP tool
        let _temp_config_file = if let ToolRequest::ImapReader(ref mut req) = tool_request {
            if req.config_path.is_none() {
                tracing::debug!(
                    host = %self.imap_config.host,
                    username = %self.imap_config.username,
                    "Injecting IMAP credentials into tool call"
                );

                // Create temporary config file with credentials
                let mut temp_file =
                    NamedTempFile::new().context("Failed to create temporary config file")?;

                let config_json = serde_json::json!({
                    "host": self.imap_config.host,
                    "port": self.imap_config.port,
                    "username": self.imap_config.username,
                    "password": self.imap_config.password,
                });

                temp_file
                    .write_all(config_json.to_string().as_bytes())
                    .context("Failed to write IMAP config to temp file")?;
                temp_file.flush().context("Failed to flush temp file")?;

                // Get the path and inject it into the request
                let config_path = temp_file
                    .path()
                    .to_str()
                    .context("Failed to get temp file path")?
                    .to_string();

                req.config_path = Some(config_path);

                tracing::debug!(
                    config_path = ?req.config_path,
                    "Injected IMAP config file path"
                );

                Some(temp_file) // Keep file alive for the duration of tool execution
            } else {
                None
            }
        } else {
            None
        };

        let mut tool_call_record = StorageToolCall {
            id: None,
            session_id,
            message_id,
            tool_call_id: tool_call.id().to_string(),
            tool_name: tool_call.name().to_string(),
            request: tool_call.arguments().clone(),
            response: None,
            status: ToolCallStatus::Pending,
            execution_time_ms: None,
            created_at: chrono::Utc::now().timestamp(),
            completed_at: None,
            error_details: None,
        };
        let call_id = self
            .storage
            .create_tool_call(tool_call_record.clone())
            .await?;
        tool_call_record.id = Some(call_id);

        let start = Instant::now();
        let result: anyhow::Result<nocodo_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

        match result {
            Ok(response) => {
                let response_json = serde_json::to_value(&response)?;
                tool_call_record.complete(response_json.clone(), execution_time);
                self.storage.update_tool_call(tool_call_record).await?;

                let result_text = crate::format_tool_response(&response);
                let message_to_llm = format!("Tool {} result:\n{}", tool_call.name(), result_text);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    execution_time_ms = execution_time,
                    "Tool execution completed successfully"
                );

                let tool_message = StorageMessage {
                    id: None,
                    session_id,
                    role: MessageRole::Tool,
                    content: message_to_llm,
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.storage.create_message(tool_message).await?;
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                tool_call_record.fail(error_msg.clone());
                self.storage.update_tool_call(tool_call_record).await?;

                let error_message_to_llm =
                    format!("Tool {} failed: {}", tool_call.name(), error_msg);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    error = %error_msg,
                    "Tool execution failed"
                );

                let tool_message = StorageMessage {
                    id: None,
                    session_id,
                    role: MessageRole::Tool,
                    content: error_message_to_llm,
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.storage.create_message(tool_message).await?;
            }
        }

        Ok(())
    }

    #[cfg(test)]
    pub fn new_for_testing(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        host: String,
        port: u16,
        username: String,
        password: String,
    ) -> Self {
        Self {
            client,
            database,
            tool_executor,
            imap_config: ImapConfig {
                host,
                port,
                username,
                password,
            },
            system_prompt: IMAP_EMAIL_AGENT_SYSTEM_PROMPT.to_string(),
        }
    }
}

#[async_trait]
impl<S: AgentStorage> Agent for ImapEmailAgent<S> {
    fn objective(&self) -> &str {
        "Analyze and manage emails via IMAP"
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![AgentTool::ImapReader]
    }

    fn settings_schema(&self) -> AgentSettingsSchema {
        Self::static_settings_schema().unwrap_or_else(|| AgentSettingsSchema {
            agent_name: "IMAP Email Agent".to_string(),
            section_name: "imap_email".to_string(),
            settings: vec![],
        })
    }

    fn static_settings_schema() -> Option<AgentSettingsSchema>
    where
        Self: Sized,
    {
        Some(AgentSettingsSchema {
            agent_name: "IMAP Email Agent".to_string(),
            section_name: "imap_email".to_string(),
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
                    setting_type: SettingType::Text,
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

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        let user_message = StorageMessage {
            id: None,
            session_id,
            role: MessageRole::User,
            content: user_prompt.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        self.storage.create_message(user_message).await?;

        let tools = self.get_tool_definitions();

        let mut iteration = 0;
        let max_iterations = 30;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                let error = "Maximum iteration limit reached";
                let mut session = self.get_session(session_id).await?;
                session.status = SessionStatus::Failed;
                session.error = Some(error.to_string());
                session.ended_at = Some(chrono::Utc::now().timestamp());
                self.storage.update_session(session).await?;
                return Err(anyhow::anyhow!(error));
            }

            let messages = self.build_messages(session_id).await?;

            let request = CompletionRequest {
                messages,
                max_tokens: 4000,
                model: self.client.model_name().to_string(),
                system: Some(self.system_prompt()),
                temperature: Some(0.7),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            let response = self.client.complete(request).await?;

            let text = extract_text_from_content(&response.content);

            let text_to_save = if text.is_empty() && response.tool_calls.is_some() {
                "[Using tools]".to_string()
            } else {
                text.clone()
            };

            let assistant_message = StorageMessage {
                id: None,
                session_id,
                role: MessageRole::Assistant,
                content: text_to_save,
                created_at: chrono::Utc::now().timestamp(),
            };
            let message_id = self.storage.create_message(assistant_message).await?;

            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    let mut session = self.get_session(session_id).await?;
                    session.status = SessionStatus::Completed;
                    session.result = Some(text.clone());
                    session.ended_at = Some(chrono::Utc::now().timestamp());
                    self.storage.update_session(session).await?;
                    return Ok(text);
                }

                for tool_call in tool_calls {
                    self.execute_tool_call(session_id, Some(message_id), &tool_call)
                        .await?;
                }
            } else {
                let mut session = self.get_session(session_id).await?;
                session.status = SessionStatus::Completed;
                session.result = Some(text.clone());
                session.ended_at = Some(chrono::Utc::now().timestamp());
                self.storage.update_session(session).await?;
                return Ok(text);
            }
        }
    }
}

fn extract_text_from_content(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
