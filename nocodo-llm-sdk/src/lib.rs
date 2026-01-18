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
//!         ClaudeContentBlock::ToolUse { .. } => println!("Response: [Tool use]"),
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
//!     .user_message("Hello, GLM!")
//!     .send()
//!     .await?;
//!
//!     println!("Response: {}", response.choices[0].message.get_text());
//!     Ok(())
//! }
//! ```
//!
//! ## OpenAI Chat Completions Example
//!
//! ```rust,no_run
//! use nocodo_llm_sdk::openai::OpenAIClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = OpenAIClient::new("your-openai-api-key")?;
//!     let response = client
//!         .message_builder()
//!         .model("gpt-4o")
//!         .max_completion_tokens(1024)
//!         .user_message("Hello, GPT!")
//!         .send()
//!         .await?;
//!
//!     println!("Response: {}", response.choices[0].message.content);
//!     Ok(())
//! }
//! ```
//!
//! ## OpenAI Responses API Example (GPT-5.1-Codex)
//!
//! ```rust,no_run
//! use nocodo_llm_sdk::openai::OpenAIClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = OpenAIClient::new("your-openai-api-key")?;
//!     let response = client
//!         .response_builder()
//!         .model("gpt-5.1-codex")
//!         .input("Write a Python function to calculate fibonacci numbers")
//!         .send()
//!         .await?;
//!
//!     // Extract text from the response
//!     for item in &response.output {
//!         if item.item_type == "message" {
//!             if let Some(content_blocks) = &item.content {
//!                 for block in content_blocks {
//!                     if block.content_type == "output_text" {
//!                         println!("Response: {}", block.text);
//!                     }
//!                 }
//!             }
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Voyage AI Embeddings Example
//!
//! ```rust,no_run
//! use nocodo_llm_sdk::voyage::{VoyageClient, VoyageInputType};
//! use nocodo_llm_sdk::models::voyage::VOYAGE_4_LITE;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = VoyageClient::new("your-voyage-api-key")?;
//!     let response = client
//!         .embedding_builder()
//!         .model(VOYAGE_4_LITE)
//!         .input(vec!["Hello, world!", "Text embeddings are useful"])
//!         .input_type(VoyageInputType::Document)
//!         .output_dimension(1024)
//!         .send()
//!         .await?;
//!
//!     for embedding in &response.data {
//!         println!("Embedding {}: {} dimensions", embedding.index, embedding.embedding.len());
//!     }
//!     Ok(())
//! }
//! ```

pub mod claude;
pub mod client;
pub mod error;
pub mod gemini;
pub mod glm;
pub mod grok;
pub mod model_metadata;
pub mod models;
pub mod openai;
pub mod providers;
pub mod tools;
pub mod types;
pub mod voyage;

// Provider-specific exports
pub use gemini::GeminiClient;
pub use glm::cerebras::CerebrasGlmClient;
pub use glm::zen::ZenGlmClient;
pub use grok::xai::XaiGrokClient;
pub use grok::zen::ZenGrokClient;

// Tool exports
pub use tools::{Tool, ToolCall, ToolChoice, ToolResult};

// Model constants exports
pub use models::*;

// Backwards compatibility aliases
#[deprecated(since = "0.2.0", note = "Use XaiGrokClient explicitly")]
pub use grok::xai::XaiGrokClient as GrokClient;

#[deprecated(since = "0.2.0", note = "Use CerebrasGlmClient explicitly")]
pub use glm::cerebras::CerebrasGlmClient as GlmClient;

#[cfg(test)]
mod tests {
    use crate::claude::{
        client::ClaudeClient,
        types::{ClaudeContentBlock, ClaudeMessage, ClaudeRole},
    };
    use crate::glm::{
        cerebras::CerebrasGlmClient,
        types::{GlmMessage, GlmRole},
    };
    use crate::grok::{
        types::{GrokMessage, GrokRole},
        xai::XaiGrokClient,
    };
    use crate::openai::{
        client::OpenAIClient,
        types::{OpenAIMessage, OpenAIRole},
    };
    use crate::voyage::client::VoyageClient;

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
            ClaudeContentBlock::ToolUse { .. } => panic!("Unexpected tool use in test"),
        }
    }

    #[test]
    fn test_xai_grok_client_creation() {
        let client = XaiGrokClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_xai_grok_client_creation_empty_key() {
        let client = XaiGrokClient::new("");
        assert!(client.is_err());
    }

    #[test]
    fn test_xai_grok_message_builder() {
        let client = XaiGrokClient::new("test-key").unwrap();
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
    fn test_cerebras_glm_client_creation() {
        let client = CerebrasGlmClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_cerebras_glm_client_creation_empty_key() {
        let client = CerebrasGlmClient::new("");
        assert!(client.is_err());
    }

    #[test]
    fn test_cerebras_glm_message_builder() {
        let client = CerebrasGlmClient::new("test-key").unwrap();
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
        assert_eq!(message.content, Some("Hello".to_string()));
        assert_eq!(message.get_text(), "Hello");
    }

    #[test]
    fn test_openai_client_creation() {
        let client = OpenAIClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_openai_client_creation_empty_key() {
        let client = OpenAIClient::new("");
        assert!(client.is_err());
    }

    #[test]
    fn test_openai_message_builder() {
        let client = OpenAIClient::new("test-key").unwrap();
        let _builder = client
            .message_builder()
            .model("gpt-5.1")
            .max_completion_tokens(100)
            .user_message("Hello");

        // The builder should be created successfully
        assert!(true); // Builder creation succeeded
    }

    #[test]
    fn test_openai_message_creation() {
        let message = OpenAIMessage::user("Hello");
        assert_eq!(message.role, OpenAIRole::User);
        assert_eq!(message.content, "Hello");
    }

    #[test]
    fn test_voyage_client_creation() {
        let client = VoyageClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_voyage_client_creation_empty_key() {
        let client = VoyageClient::new("");
        assert!(client.is_err());
    }

    #[test]
    fn test_voyage_embedding_builder() {
        let client = VoyageClient::new("test-key").unwrap();
        let _builder = client
            .embedding_builder()
            .model("voyage-4-lite")
            .input("Hello");

        // The builder should be created successfully
        assert!(true); // Builder creation succeeded
    }
}
