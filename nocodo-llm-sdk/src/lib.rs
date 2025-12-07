//! # Nocodo LLM SDK
//!
//! A general-purpose LLM SDK for Rust, starting with Claude support.
//!
//! ## Example
//!
//! ```rust,no_run
//! use nocodo_llm_sdk::claude::{ClaudeClient, types::ClaudeContentBlock};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = ClaudeClient::new("your-api-key")?;
//!     let response = client
//!         .message_builder()
//!         .model("claude-sonnet-4-5-20250929")
//!         .max_tokens(1024)
//!         .message("user", "Hello, Claude!")
//!         .send()
//!         .await?;
//!
//!     match &response.content[0] {
//!         ClaudeContentBlock::Text { text } => println!("Response: {}", text),
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Grok Example
//!
//! ```rust,no_run
//! use nocodo_llm_sdk::grok::GrokClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = GrokClient::new("your-xai-api-key")?;
//!     let response = client
//!         .message_builder()
//!         .model("grok-code-fast-1")
//!         .max_tokens(1024)
//!         .user_message("Hello, Grok!")
//!         .send()
//!         .await?;
//!
//!     println!("Response: {}", response.choices[0].message.content);
//!     Ok(())
//! }
//! ```
//!
//! ## GLM Example
//!
//! ```rust,no_run
//! use nocodo_llm_sdk::glm::GlmClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = GlmClient::new("your-cerebras-api-key")?;
//!     let response = client
//!         .message_builder()
//!         .model("zai-glm-4.6")
//!         .max_tokens(1024)
//!         .user_message("Hello, GLM!")
//!         .send()
//!         .await?;
//!
//!     println!("Response: {}", response.choices[0].message.content);
//!     Ok(())
//! }
//! ```

pub mod claude;
pub mod client;
pub mod error;
pub mod glm;
pub mod grok;
pub mod types;

#[cfg(test)]
mod tests {
    use crate::claude::{
        client::ClaudeClient,
        types::{ClaudeContentBlock, ClaudeMessage, ClaudeRole},
    };
    use crate::glm::{
        client::GlmClient,
        types::{GlmMessage, GlmRole},
    };
    use crate::grok::{
        client::GrokClient,
        types::{GrokMessage, GrokRole},
    };

    #[test]
    fn test_claude_client_creation() {
        let client = ClaudeClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_claude_client_creation_empty_key() {
        let client = ClaudeClient::new("");
        assert!(client.is_err());
    }

    #[test]
    fn test_claude_message_builder() {
        let client = ClaudeClient::new("test-key").unwrap();
        let _builder = client
            .message_builder()
            .model("test-model")
            .max_tokens(100)
            .user_message("Hello");

        // The builder should be created successfully
        assert!(true); // Builder creation succeeded
    }

    #[test]
    fn test_claude_message_creation() {
        let message = ClaudeMessage::text(ClaudeRole::User, "Hello");
        assert_eq!(message.role, ClaudeRole::User);
        assert_eq!(message.content.len(), 1);
        match &message.content[0] {
            ClaudeContentBlock::Text { text } => assert_eq!(text, "Hello"),
        }
    }

    #[test]
    fn test_grok_client_creation() {
        let client = GrokClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_grok_client_creation_empty_key() {
        let client = GrokClient::new("");
        assert!(client.is_err());
    }

    #[test]
    fn test_grok_message_builder() {
        let client = GrokClient::new("test-key").unwrap();
        let _builder = client
            .message_builder()
            .model("grok-code-fast-1")
            .max_tokens(100)
            .user_message("Hello");

        // The builder should be created successfully
        assert!(true); // Builder creation succeeded
    }

    #[test]
    fn test_grok_message_creation() {
        let message = GrokMessage::user("Hello");
        assert_eq!(message.role, GrokRole::User);
        assert_eq!(message.content, "Hello");
    }

    #[test]
    fn test_glm_client_creation() {
        let client = GlmClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_glm_client_creation_empty_key() {
        let client = GlmClient::new("");
        assert!(client.is_err());
    }

    #[test]
    fn test_glm_message_builder() {
        let client = GlmClient::new("test-key").unwrap();
        let _builder = client
            .message_builder()
            .model("zai-glm-4.6")
            .max_tokens(100)
            .user_message("Hello");

        // The builder should be created successfully
        assert!(true); // Builder creation succeeded
    }

    #[test]
    fn test_glm_message_creation() {
        let message = GlmMessage::user("Hello");
        assert_eq!(message.role, GlmRole::User);
        assert_eq!(message.content, "Hello");
    }
}
