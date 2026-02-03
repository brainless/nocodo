pub mod bash;
pub mod core;
pub mod filesystem;
pub mod grep;
#[cfg(feature = "sqlite")]
pub mod hackernews;
pub mod imap;
pub mod pdftotext;
#[cfg(feature = "sqlite")]
pub mod sqlite_reader;

// Re-export commonly used types
pub use bash::{BashRequest, BashResponse};
pub use core::{ToolErrorResponse, ToolRequest, ToolResponse};
pub use filesystem::{
    ApplyPatchFileChange, ApplyPatchRequest, ApplyPatchResponse, FileInfo, FileType,
    ListFilesRequest, ListFilesResponse, ReadFileRequest, ReadFileResponse, WriteFileRequest,
    WriteFileResponse,
};
pub use grep::{GrepMatch, GrepRequest, GrepResponse};
#[cfg(feature = "sqlite")]
pub use hackernews::{DownloadState, FetchMode, HackerNewsRequest, HackerNewsResponse, StoryType};
pub use imap::{ImapOperation, ImapReaderRequest, ImapReaderResponse, SearchCriteria};
pub use pdftotext::{PdfToTextRequest, PdfToTextResponse};
#[cfg(feature = "sqlite")]
pub use sqlite_reader::{Sqlite3ReaderRequest, Sqlite3ReaderResponse, SqliteMode};

// Re-export user interaction types from shared-types
pub use shared_types::{
    AskUserRequest, AskUserResponse, QuestionType, UserQuestion, UserQuestionResponse,
};
