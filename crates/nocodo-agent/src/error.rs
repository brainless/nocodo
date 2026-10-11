use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "environment variable {var} is not set; set it to your {provider} API key and try again"
    )]
    MissingApiKey { provider: String, var: &'static str },
    #[error("unknown provider `{provider}` ({reason}); valid selections: {valid}")]
    UnknownProvider {
        provider: String,
        reason: String,
        valid: String,
    },
    #[error("cannot build the model for provider `{provider}`: {message}")]
    ModelBuild { provider: String, message: String },
    #[error("could not write agent log {}: {message}", path.display())]
    Log { path: PathBuf, message: String },
    #[error("the model did not return usable output: {message}")]
    Extraction {
        message: String,
        /// The run whose `.agent-logs/<run_id>.jsonl` shows what happened.
        run_id: String,
        /// Waits spent retrying errors the provider marked retryable.
        provider_retries: u64,
    },
}
