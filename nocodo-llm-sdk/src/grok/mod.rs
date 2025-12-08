pub mod builder;
pub mod types;
pub mod xai;
pub mod zen;

pub use builder::GrokMessageBuilder;
pub use types::*;

// Re-export for convenience
pub use xai::*;
pub use zen::*;

// Type alias for backwards compatibility
#[deprecated(since = "0.2.0", note = "Use xai::XaiGrokClient explicitly")]
pub type GrokClient = xai::XaiGrokClient;
