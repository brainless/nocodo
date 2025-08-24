use std::fmt;

/// Main error type for the nocodo CLI
#[derive(Debug)]
pub enum CliError {
    /// Configuration-related errors
    Config(String),
    /// File I/O errors
    Io(std::io::Error),
    /// Project analysis errors
    Analysis(String),
    /// Command execution errors
    Command(String),
    /// Communication with Manager daemon errors
    Communication(String),
    /// Generic errors from anyhow
    Other(anyhow::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Config(msg) => write!(f, "Configuration error: {msg}"),
            CliError::Io(err) => write!(f, "I/O error: {err}"),
            CliError::Analysis(msg) => write!(f, "Analysis error: {msg}"),
            CliError::Command(msg) => write!(f, "Command error: {msg}"),
            CliError::Communication(msg) => write!(f, "Communication error: {msg}"),
            CliError::Other(err) => write!(f, "Error: {err}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::Io(err) => Some(err),
            CliError::Other(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl CliError {
    /// Get the exit code for this error type
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Config(_) => 2,
            CliError::Io(_) => 3,
            CliError::Analysis(_) => 4,
            CliError::Command(_) => 5,
            CliError::Communication(_) => 7,
            CliError::Other(_) => 1,
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::Io(err)
    }
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> Self {
        CliError::Other(err)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError::Other(err.into())
    }
}

impl From<serde_yaml::Error> for CliError {
    fn from(err: serde_yaml::Error) -> Self {
        CliError::Other(err.into())
    }
}

