use nocodo_llm_sdk::tools::Tool;
use nocodo_tools::types::filesystem::*;
use nocodo_tools::types::{BashRequest, GrepRequest};
use shared_types::user_interaction::*;

fn default_true() -> bool {
    true
}

/// Create tool definitions for LLM using manager-models types
pub fn create_tool_definitions() -> Vec<Tool> {
    let sqlite_schema = serde_json::json!({
        "type": "object",
        "required": ["query"],
        "properties": {
            "query": {
                "type": "string",
                "description": "SQL query to execute. Use SELECT to retrieve data, or PRAGMA statements to inspect database schema. PRAGMA commands include: table_list (list tables), table_info(table_name) (column info), index_list(table_name) (indexes), foreign_key_list(table_name) (foreign keys)."
            },
            "limit": {"type": "integer", "description": "Maximum number of rows to return. Defaults to 100, maximum 1000."}
        }
    });

    let sqlite_tool = Tool::from_json_schema(
        "sqlite3_reader".to_string(),
        "Read-only SQLite database tool. Use SELECT queries to retrieve data and PRAGMA statements to inspect database schema (tables, columns, indexes, foreign keys). The database path is pre-configured.".to_string(),
        sqlite_schema,
    ).expect("Failed to create sqlite3_reader tool schema");

    let imap_schema = serde_json::json!({
        "type": "object",
        "required": ["operation"],
        "properties": {
            "config_path": {
                "type": "string",
                "description": "Optional path to IMAP config file. If not provided, credentials from agent settings are used."
            },
            "operation": {
                "type": "object",
                "description": "The IMAP operation to execute. Each operation type has its own schema.",
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["type"],
                        "properties": {
                            "type": {"const": "list_mailboxes"},
                            "pattern": {
                                "type": "string",
                                "description": "Mailbox pattern (e.g., '*' for all, 'INBOX/*' for INBOX subfolders)"
                            }
                        }
                    },
                    {
                        "type": "object",
                        "required": ["type", "mailbox"],
                        "properties": {
                            "type": {"const": "mailbox_status"},
                            "mailbox": {
                                "type": "string",
                                "description": "Mailbox name (e.g., 'INBOX')"
                            }
                        }
                    },
                    {
                        "type": "object",
                        "required": ["type", "mailbox", "criteria"],
                        "properties": {
                            "type": {"const": "search"},
                            "mailbox": {
                                "type": "string",
                                "description": "Mailbox to search (e.g., 'INBOX')"
                            },
                            "criteria": {
                                "type": "object",
                                "description": "Search criteria",
                                "properties": {
                                    "from": {"type": "string", "description": "Filter by sender email/name"},
                                    "to": {"type": "string", "description": "Filter by recipient email/name"},
                                    "subject": {"type": "string", "description": "Filter by subject text"},
                                    "since_date": {"type": "string", "description": "Emails on or after date (RFC3501 format: DD-MMM-YYYY, e.g., '15-JAN-2026')"},
                                    "before_date": {"type": "string", "description": "Emails before date (RFC3501 format)"},
                                    "unseen_only": {"type": "boolean", "description": "Only return unread emails", "default": false},
                                    "raw_query": {"type": "string", "description": "Raw IMAP search query (advanced users only)"}
                                }
                            },
                            "limit": {"type": "integer", "description": "Maximum number of UIDs to return"}
                        }
                    },
                    {
                        "type": "object",
                        "required": ["type", "mailbox", "message_uids"],
                        "properties": {
                            "type": {"const": "fetch_headers"},
                            "mailbox": {"type": "string", "description": "Mailbox name"},
                            "message_uids": {
                                "type": "array",
                                "items": {"type": "integer"},
                                "description": "List of message UIDs to fetch"
                            }
                        }
                    },
                    {
                        "type": "object",
                        "required": ["type", "mailbox", "message_uid"],
                        "properties": {
                            "type": {"const": "fetch_email"},
                            "mailbox": {"type": "string", "description": "Mailbox name"},
                            "message_uid": {"type": "integer", "description": "Message UID to fetch"},
                            "include_html": {"type": "boolean", "description": "Include HTML body if available", "default": false},
                            "include_text": {"type": "boolean", "description": "Include text body", "default": true}
                        }
                    }
                ]
            },
            "timeout_seconds": {"type": "integer", "description": "Operation timeout in seconds. Defaults to 30."}
        }
    });

    let imap_tool = Tool::from_json_schema(
        "imap_reader".to_string(),
        "Read emails from IMAP mailboxes. Supports listing mailboxes, searching emails, fetching headers, and downloading email content. Always fetch headers first to analyze metadata before downloading full emails. This tool is READ-ONLY.".to_string(),
        imap_schema,
    ).expect("Failed to create imap_reader tool schema");

    let pdftotext_schema = serde_json::json!({
        "type": "object",
        "required": ["file_path"],
        "properties": {
            "file_path": {
                "type": "string",
                "description": "Path to the PDF file to extract text from"
            },
            "output_path": {
                "type": "string",
                "description": "Optional output file path. If not specified, text is returned in the response"
            },
            "preserve_layout": {
                "type": "boolean",
                "description": "Preserve original physical layout (default: true). Uses pdftotext -layout flag",
                "default": true
            },
            "first_page": {
                "type": "integer",
                "description": "First page to convert (optional, 1-based index)"
            },
            "last_page": {
                "type": "integer",
                "description": "Last page to convert (optional, 1-based index)"
            },
            "encoding": {
                "type": "string",
                "description": "Output text encoding (default: UTF-8)"
            },
            "no_page_breaks": {
                "type": "boolean",
                "description": "Don't insert page breaks between pages (default: false)",
                "default": false
            }
        }
    });

    let pdftotext_tool = Tool::from_json_schema(
        "pdftotext".to_string(),
        "Extract text from PDF files using pdftotext. Supports layout preservation, page range selection, and various encoding options. Use preserve_layout=true (default) to maintain formatting.".to_string(),
        pdftotext_schema,
    ).expect("Failed to create pdftotext tool schema");

    vec![
        Tool::from_type::<ListFilesRequest>()
            .name("list_files")
            .description("List files and directories in a given path")
            .build(),
        Tool::from_type::<ReadFileRequest>()
            .name("read_file")
            .description("Read the contents of a file")
            .build(),
        Tool::from_type::<WriteFileRequest>()
            .name("write_file")
            .description("Write or modify a file")
            .build(),
        Tool::from_type::<GrepRequest>()
            .name("grep")
            .description("Search for patterns in files using grep")
            .build(),
        Tool::from_type::<ApplyPatchRequest>()
            .name("apply_patch")
            .description("Apply a patch to create, modify, delete, or move multiple files")
            .build(),
        Tool::from_type::<BashRequest>()
            .name("bash")
            .description("Execute bash commands with timeout and permission checking")
            .build(),
        Tool::from_type::<AskUserRequest>()
            .name("ask_user")
            .description(
                "Ask the user a list of questions to gather information or confirm actions",
            )
            .build(),
        sqlite_tool,
        imap_tool,
        pdftotext_tool,
    ]
}

/// Get tool definition by name
pub fn get_tool_definition(tool_name: &str) -> Option<Tool> {
    create_tool_definitions()
        .into_iter()
        .find(|tool| tool.name() == tool_name)
}
