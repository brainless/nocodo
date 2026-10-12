use nocodo_orchestrator::OrchestratorError;
use nocodo_runtime::RuntimeError;

/// Why an operation failed. `Display` is the plain-language message a front
/// end shows as is.
#[derive(Debug)]
pub enum EngineError {
    Orchestrator(OrchestratorError),
    Runtime(RuntimeError),
    /// A list (table) that is not in the schema.
    NotFound(String),
    /// Anything else (unreadable settings file, no async runtime).
    Message(String),
    /// Another operation is writing to the project.
    Busy {
        running: Option<String>,
        pid: Option<u32>,
        since_secs: Option<u64>,
    },
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Orchestrator(e) => write!(f, "{e}"),
            Self::Runtime(e) => write!(f, "{e}"),
            Self::Message(m) | Self::NotFound(m) => write!(f, "{m}"),
            Self::Busy {
                running,
                pid,
                since_secs,
            } => {
                write!(f, "This project is busy: ")?;
                match running {
                    Some(op) => write!(f, "`{op}` is running")?,
                    None => write!(f, "another change is running")?,
                }
                if let Some(pid) = pid {
                    write!(f, " (process {pid})")?;
                }
                if let Some(s) = since_secs {
                    write!(f, ", started {s} seconds ago")?;
                }
                write!(
                    f,
                    ". Wait for it to finish and try again. Reading (inspect, questions) still works."
                )
            }
        }
    }
}

impl std::error::Error for EngineError {}

impl From<OrchestratorError> for EngineError {
    fn from(e: OrchestratorError) -> Self {
        Self::Orchestrator(e)
    }
}

impl From<RuntimeError> for EngineError {
    fn from(e: RuntimeError) -> Self {
        Self::Runtime(e)
    }
}
