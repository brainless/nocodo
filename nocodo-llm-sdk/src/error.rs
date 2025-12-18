use thiserror::Error;

/// Comprehensive error types for LLM operations
#[derive(Error, Debug)]
pub enum LlmError {
    /// Authentication failed (HTTP 401)
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// Rate limit exceeded (HTTP 429)
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Option<u64>,
    },

    /// Invalid request parameters (HTTP 400)
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    /// API error with status code (HTTP 4xx/5xx except above)
    #[error("API error (status {status}): {message}")]
    Api { status: u16, message: String },

    /// Network or connection error
    #[error("Network error: {source}")]
    Network {
        #[from]
        source: reqwest::Error,
    },

    /// JSON parsing or serialization error
    #[error("Parse error: {source}")]
    Parse {
        #[from]
        source: serde_json::Error,
    },

    /// Generic error for unexpected cases
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Invalid tool schema
    #[error("Invalid tool schema: {message}")]
    InvalidToolSchema { message: String },

    /// Failed to parse tool arguments
    #[error("Failed to parse tool arguments for {tool_name}: {source}")]
    ToolArgumentParse {
        tool_name: String,
        source: serde_json::Error,
    },

    /// Tool execution failed
    #[error("Tool execution failed: {message}")]
    ToolExecutionFailed { message: String },

    /// Feature not supported
    #[error("Not supported: {message}")]
    NotSupported { message: String },
}

impl LlmError {
    /// Create an authentication error
    pub fn authentication<S: Into<String>>(message: S) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit<S: Into<String>>(message: S, retry_after: Option<u64>) -> Self {
        Self::RateLimit {
            message: message.into(),
            retry_after,
        }
    }

    /// Create an invalid request error
    pub fn invalid_request<S: Into<String>>(message: S) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    /// Create an API error
    pub fn api_error(status: u16, message: String) -> Self {
        Self::Api { status, message }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a not supported error
    pub fn not_supported<S: Into<String>>(message: S) -> Self {
        Self::NotSupported {
            message: message.into(),
        }
    }
}
