pub mod builder;
pub mod types;
pub mod cerebras;
pub mod zen;

pub use builder::GlmMessageBuilder;
pub use types::*;

// Re-export for convenience
pub use cerebras::*;
pub use zen::*;

// Type alias for backwards compatibility
#[deprecated(since = "0.2.0", note = "Use cerebras::CerebrasGlmClient explicitly")]
pub type GlmClient = cerebras::CerebrasGlmClient;
