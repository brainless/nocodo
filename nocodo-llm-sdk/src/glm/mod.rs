pub mod builder;
pub mod tools;
pub mod types;
pub mod cerebras;
pub mod zen;
pub mod zai;

pub use builder::GlmMessageBuilder;
pub use tools::GlmToolFormat;
pub use types::*;

// Re-export for convenience
pub use cerebras::*;
pub use zen::*;
pub use zai::*;

// Type alias for backwards compatibility
#[deprecated(since = "0.2.0", note = "Use cerebras::CerebrasGlmClient explicitly")]
pub type GlmClient = cerebras::CerebrasGlmClient;

// Re-export GLM model constants
pub use crate::models::glm::*;
