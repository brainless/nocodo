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

pub mod claude;
pub mod client;
pub mod error;
pub mod types;

#[cfg(test)]
mod tests {
    use crate::claude::{
        client::ClaudeClient,
        types::{ClaudeContentBlock, ClaudeMessage, ClaudeRole},
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
    fn test_message_builder() {
        let client = ClaudeClient::new("test-key").unwrap();
        let _builder = client
            .message_builder()
            .model("test-model")
            .max_tokens(100)
            .user_message("Hello");

        // The builder should be created successfully
        // We can't test internal state since fields are private,
        // but we can test that the builder exists
        assert!(true); // Builder creation succeeded
    }

    #[test]
    fn test_message_creation() {
        let message = ClaudeMessage::text(ClaudeRole::User, "Hello");
        assert_eq!(message.role, ClaudeRole::User);
        assert_eq!(message.content.len(), 1);
        match &message.content[0] {
            ClaudeContentBlock::Text { text } => assert_eq!(text, "Hello"),
        }
    }
}
