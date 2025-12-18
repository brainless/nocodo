use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// File operation models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileType {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub path: String,                // relative path
    pub absolute: String,            // absolute path
    pub file_type: FileType,         // enum: File, Directory
    pub ignored: bool,               // whether file is ignored by .gitignore
    pub is_directory: bool,          // computed from file_type
    pub size: Option<u64>,           // file size in bytes, None for directories
    pub modified_at: Option<String>, // ISO 8601 timestamp, None for directories
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListRequest {
    pub project_id: Option<i64>,
    pub path: Option<String>, // Relative path within project, defaults to root
    pub git_branch: Option<String>, // Git branch/worktree to use, defaults to current branch
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListResponse {
    pub files: Vec<FileInfo>, // List of files and directories
    pub current_path: String, // Current directory being listed
    pub total_files: u32,     // Total number of files found
    pub truncated: bool,      // Whether results were limited to 100
    pub limit: u32,           // Maximum files returned (100)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileCreateRequest {
    pub project_id: i64,
    pub path: String,            // Relative path within project
    pub content: Option<String>, // None for directories
    pub is_directory: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileUpdateRequest {
    pub project_id: i64,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileContentResponse {
    pub path: String,
    pub content: String,
    pub modified_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileResponse {
    pub file: FileInfo,
}

/// List files and directories in a given path
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_files: Option<u32>,
}

impl ListFilesRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list files from"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to list files recursively",
                    "default": false
                },
                "include_hidden": {
                    "type": "boolean",
                    "description": "Whether to include hidden files",
                    "default": false
                },
                "max_files": {
                    "type": "number",
                    "description": "Maximum number of files to return (default: 1000)",
                    "default": 1000
                }
            },
            "required": ["path"]
        })
    }
}

/// Read the contents of a file
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,
}

impl ReadFileRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to read"
                },
                "max_size": {
                    "type": "number",
                    "description": "Maximum number of bytes to read",
                    "default": 10000
                }
            },
            "required": ["path"]
        })
    }
}

/// Write or modify a file. Supports two modes: 1) Full write with 'content' parameter, 2) Search & replace with 'search' and 'replace' parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteFileRequest {
    /// The file path to write to
    pub path: String,

    /// Full content to write (required for full write, omit for search-replace)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Whether to create parent directories if they don't exist
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_dirs: Option<bool>,

    /// Whether to append to file instead of overwriting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,

    /// Text to search for (for search-and-replace operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,

    /// Text to replace the search text with
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<String>,

    /// Whether to create file if it doesn't exist
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_if_not_exists: Option<bool>,
}

impl WriteFileRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to write to"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to file"
                },
                "create_dirs": {
                    "type": "boolean",
                    "description": "Whether to create parent directories if they don't exist",
                    "default": false
                },
                "append": {
                    "type": "boolean",
                    "description": "Whether to append to file instead of overwriting",
                    "default": false
                },
                "search": {
                    "type": "string",
                    "description": "Text to search for (for search and replace operations)"
                },
                "replace": {
                    "type": "string",
                    "description": "Text to replace to search text with"
                },
                "create_if_not_exists": {
                    "type": "boolean",
                    "description": "Whether to create file if it doesn't exist",
                    "default": false
                }
            },
            "required": ["path"]
        })
    }

    /// Validate request parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.content.is_none() && (self.search.is_none() || self.replace.is_none()) {
            return Err("Either 'content' must be provided, or both 'search' and 'replace' must be provided".to_string());
        }
        if self.search.is_some() != self.replace.is_some() {
            return Err("Both 'search' and 'replace' must be provided together".to_string());
        }
        Ok(())
    }
}

/// Apply a patch to create, modify, delete, or move multiple files
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyPatchRequest {
    /// The patch content in unified diff format
    ///
    /// Format:
    /// ```text
    /// *** Begin Patch
    /// *** Add File: path/to/new.txt
    /// +line content
    /// *** Update File: path/to/existing.txt
    /// @@ optional context
    /// -old line
    /// +new line
    /// *** Delete File: path/to/remove.txt
    /// *** End Patch
    /// ```
    ///
    /// Supports:
    /// - Add File: Create new files with + prefixed lines
    /// - Update File: Modify files with diff hunks (- for removed, + for added)
    /// - Delete File: Remove files
    /// - Move to: Rename files (after Update File header)
    /// - @@ context headers for targeting specific code blocks
    ///
    /// All file paths must be relative to project root.
    pub patch: String,
}

/// Apply patch file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchFileChange {
    pub path: String,
    pub operation: String, // "add", "update", "delete", "move"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_path: Option<String>, // For move operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unified_diff: Option<String>, // For update operations
}

/// Apply patch tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchResponse {
    pub success: bool,
    pub files_changed: Vec<ApplyPatchFileChange>,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub message: String,
}

// Response types for filesystem tools

/// List files tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFilesResponse {
    pub current_path: String,
    pub files: String, // Plain text tree representation
    pub total_files: u32,
    pub truncated: bool,
    pub limit: u32,
}

/// Read file tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileResponse {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub truncated: bool,
}

/// Write file tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileResponse {
    pub path: String,
    pub success: bool,
    pub bytes_written: u64,
    pub created: bool,
    pub modified: bool,
}