//! Google Gemini API client and types
//!
//! Supports Gemini 3 Pro and Gemini 3 Flash models with reasoning capabilities.

pub mod builder;
pub mod client;
pub mod tools;
pub mod types;

pub use builder::MessageBuilder;
pub use client::GeminiClient;
pub use tools::GeminiToolFormat;
pub use types::*;

// Re-export model constants
pub use crate::models::gemini::*;
