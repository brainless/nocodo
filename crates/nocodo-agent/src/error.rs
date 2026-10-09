use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("environment variable {0} is not set; set it to your API key and try again")]
    MissingApiKey(&'static str),
    #[error("could not write agent log {}: {message}", path.display())]
    Log { path: PathBuf, message: String },
    #[error("the model did not return usable output: {0}")]
    Extraction(String),
}
