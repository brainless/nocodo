# Add Tesseract OCR Agent to nocodo-agents

**Status**: 📋 Not Started
**Priority**: Medium
**Created**: 2026-01-09
**Dependencies**: manager-tools task "add-command-restricted-bash-executor.md"

## Summary

Create a specialized AI agent (`TesseractAgent`) for extracting text from images using Tesseract OCR. The agent will have restricted bash access (only `tesseract` command), can clean and format extracted text using LLM capabilities, and handles various image formats and OCR options.

## Problem Statement

Users need to extract text from images (PDFs, screenshots, scanned documents) and process it:
- Manual OCR is time-consuming and error-prone
- OCR output often needs cleaning (formatting, typo correction, structure preservation)
- Different images require different OCR parameters (language, page segmentation mode)
- Security: Don't want a general-purpose agent with full bash access for OCR tasks

Currently, there's no agent specialized for OCR in nocodo-agents.

## Goals

1. **Create TesseractAgent**: Specialized agent for image-to-text extraction
2. **Restricted bash access**: Only allow `tesseract` command, no other bash access
3. **LLM-powered cleaning**: Use LLM to clean and format OCR output
4. **Flexible configuration**: Support various tesseract options (language, PSM, OEM)
5. **Pre-condition checking**: Verify tesseract is installed before running
6. **Multi-format support**: Handle PNG, JPG, PDF, TIFF, etc.
7. **Reusability**: Usable across projects requiring OCR

## Architecture Overview

### Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Bash access** | Restricted to `tesseract` only | Security - minimal permissions needed |
| **Tool set** | Bash + ReadFile + WriteFile | Bash for OCR, files for I/O |
| **Temp file handling** | Write to temp files, read results | Tesseract requires file paths |
| **LLM cleaning** | Optional post-processing step | OCR output often needs formatting/correction |
| **Pre-conditions** | Check tesseract installation | Fail fast with helpful error |
| **Base path** | Configurable working directory | Allows restricting file access |
| **Agent type** | Stateful struct | Maintains configuration and tools |

### Agent Structure

```rust
pub struct TesseractAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,  // Has restricted bash
    base_path: PathBuf,
    system_prompt: String,
}
```

### System Prompt Template

```rust
"You are a Tesseract OCR specialist. Your task is to extract text from images and
optionally clean and format the extracted text.

You have access to these tools:
1. bash - ONLY for running tesseract command
2. read_file - To read tesseract output files
3. write_file - To write cleaned results

# Tesseract Command Format

tesseract <input_image> <output_base> [options]

Common options:
- -l <lang> - Language (eng, spa, fra, deu, etc.)
- --psm <n> - Page segmentation mode:
  0 = Orientation and script detection (OSD) only
  1 = Automatic page segmentation with OSD
  3 = Fully automatic page segmentation (default)
  6 = Assume a single uniform block of text
  11 = Sparse text. Find as much text as possible
- --oem <n> - OCR Engine mode:
  0 = Legacy engine
  1 = Neural nets LSTM engine
  2 = Legacy + LSTM engines
  3 = Default (based on what is available)

# Workflow

1. Run tesseract command to extract text from image
2. Read the output file (tesseract adds .txt automatically)
3. Analyze the extracted text
4. If requested, clean and format the text:
   - Fix OCR errors (common misrecognitions)
   - Improve formatting and structure
   - Remove noise/artifacts
   - Preserve intended structure (paragraphs, lists, etc.)
5. Write the final result to output file if requested

# Example

User asks: \"Extract text from invoice.png\"

1. Run: tesseract invoice.png output -l eng --psm 6
2. Read: output.txt
3. Analyze and present the raw text
4. If asked to clean, format the text appropriately
5. Write cleaned version if requested

# Security Notes

- You can ONLY run tesseract command (no other bash commands)
- Input/output files must be within the configured base path
- Always validate file paths before reading/writing
"
```

### Execution Flow

```
User: "Extract text from document.png and clean it"
  ↓
TesseractAgent.execute()
  ↓
Agent analyzes request
  ↓
Agent calls bash tool: "tesseract document.png output -l eng"
  ↓
BashExecutor validates command against restricted permissions
  ↓ (only tesseract* allowed)
Tesseract runs, creates output.txt
  ↓
Agent calls read_file tool: "output.txt"
  ↓
Agent analyzes OCR output
  ↓
Agent uses LLM to clean and format text
  ↓
Agent calls write_file tool: "cleaned_output.txt"
  ↓
Return formatted result to user
```

## Implementation Plan

### Phase 1: Create TesseractAgent Module

#### 1.1 Create Module Structure

Create new directory and files:
```
nocodo-agents/src/
  tesseract/
    mod.rs          # Main agent implementation
```

#### 1.2 Implement TesseractAgent

**File**: `nocodo-agents/src/tesseract/mod.rs`

```rust
use crate::{database::Database, Agent, AgentTool};
use anyhow::{self, Context};
use async_trait::async_trait;
use manager_tools::{
    bash::{BashExecutor, BashPermissions},
    ToolExecutor,
};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Agent specialized in extracting text from images using Tesseract OCR
pub struct TesseractAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
    base_path: PathBuf,
    system_prompt: String,
}

impl TesseractAgent {
    /// Create a new TesseractAgent
    ///
    /// # Arguments
    /// * `client` - LLM client for AI inference
    /// * `database` - Database for session/message tracking
    /// * `base_path` - Working directory for file operations
    ///
    /// # Security
    /// The agent is configured with restricted bash access:
    /// - Only the `tesseract` command is allowed
    /// - All other bash commands are denied
    /// - File operations are restricted to base_path
    ///
    /// # Pre-conditions
    /// - Tesseract OCR must be installed on the system
    /// - Run `tesseract --version` to verify installation
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        base_path: PathBuf,
    ) -> anyhow::Result<Self> {
        // Create restricted bash permissions (only tesseract command)
        let bash_perms = BashPermissions::minimal(vec!["tesseract"]);
        let bash_executor = BashExecutor::new(bash_perms, 120);

        // Create tool executor with restricted bash
        let tool_executor = Arc::new(
            ToolExecutor::builder()
                .base_path(base_path.clone())
                .bash_executor(Some(bash_executor))
                .build(),
        );

        let system_prompt = generate_system_prompt();

        Ok(Self {
            client,
            database,
            tool_executor,
            base_path,
            system_prompt,
        })
    }

    /// Get tool definitions for this agent
    fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
        self.tools()
            .into_iter()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }

    /// Build messages from session history
    fn build_messages(&self, session_id: i64) -> anyhow::Result<Vec<Message>> {
        let db_messages = self.database.get_messages(session_id)?;

        db_messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    "system" => Role::System,
                    "tool" => Role::User,
                    _ => Role::User,
                };

                Ok(Message {
                    role,
                    content: vec![ContentBlock::Text { text: msg.content }],
                })
            })
            .collect()
    }

    /// Execute a tool call
    async fn execute_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call: &ToolCall,
    ) -> anyhow::Result<()> {
        // 1. Parse LLM tool call into typed ToolRequest
        let tool_request =
            AgentTool::parse_tool_call(tool_call.name(), tool_call.arguments().clone())?;

        // 2. Record tool call in database
        let call_id = self.database.create_tool_call(
            session_id,
            message_id,
            tool_call.id(),
            tool_call.name(),
            tool_call.arguments().clone(),
        )?;

        // 3. Execute tool
        let start = Instant::now();
        let result: anyhow::Result<manager_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

        // 4. Update database with result
        match result {
            Ok(response) => {
                let response_json = serde_json::to_value(&response)?;
                self.database
                    .complete_tool_call(call_id, response_json.clone(), execution_time)?;

                let result_text = crate::format_tool_response(&response);
                let message_to_llm = format!("Tool {} result:\n{}", tool_call.name(), result_text);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    execution_time_ms = execution_time,
                    "Tool execution completed successfully"
                );

                self.database
                    .create_message(session_id, "tool", &message_to_llm)?;
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                self.database.fail_tool_call(call_id, &error_msg)?;

                let error_message_to_llm =
                    format!("Tool {} failed: {}", tool_call.name(), error_msg);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    error = %error_msg,
                    "Tool execution failed"
                );

                self.database
                    .create_message(session_id, "tool", &error_message_to_llm)?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Agent for TesseractAgent {
    fn objective(&self) -> &str {
        "Extract text from images using Tesseract OCR and optionally clean/format the output"
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn pre_conditions(&self) -> Option<Vec<String>> {
        Some(vec![
            "Tesseract OCR must be installed on the system".to_string(),
            "Run 'tesseract --version' to verify installation".to_string(),
            "Install with: brew install tesseract (macOS) or apt-get install tesseract-ocr (Linux)".to_string(),
        ])
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![
            AgentTool::Bash,       // Only tesseract command allowed
            AgentTool::ReadFile,   // Read OCR output
            AgentTool::WriteFile,  // Write cleaned results
        ]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        // 1. Create session
        let session_id = self.database.create_session(
            "tesseract-ocr",
            self.client.provider_name(),
            self.client.model_name(),
            Some(&self.system_prompt),
            user_prompt,
        )?;

        // 2. Create initial user message
        self.database
            .create_message(session_id, "user", user_prompt)?;

        // 3. Get tool definitions
        let tools = self.get_tool_definitions();

        // 4. Execution loop (max 30 iterations)
        let mut iteration = 0;
        let max_iterations = 30;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                let error = "Maximum iteration limit reached";
                self.database.fail_session(session_id, error)?;
                return Err(anyhow::anyhow!(error));
            }

            // 5. Build request with conversation history
            let messages = self.build_messages(session_id)?;

            let request = CompletionRequest {
                messages,
                max_tokens: 4000,
                model: self.client.model_name().to_string(),
                system: Some(self.system_prompt()),
                temperature: Some(0.7),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
            };

            // 6. Call LLM
            let response = self.client.complete(request).await?;

            // 7. Extract text and save assistant message
            let text = extract_text_from_content(&response.content);
            let message_id = self
                .database
                .create_message(session_id, "assistant", &text)?;

            // 8. Check for tool calls
            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    self.database.complete_session(session_id, &text)?;
                    return Ok(text);
                }

                // 9. Execute tools
                for tool_call in tool_calls {
                    self.execute_tool_call(session_id, Some(message_id), &tool_call)
                        .await?;
                }
            } else {
                self.database.complete_session(session_id, &text)?;
                return Ok(text);
            }
        }
    }
}

/// Generate system prompt for TesseractAgent
fn generate_system_prompt() -> String {
    r#"You are a Tesseract OCR specialist. Your task is to extract text from images and
optionally clean and format the extracted text.

You have access to these tools:
1. bash - ONLY for running tesseract command
2. read_file - To read tesseract output files
3. write_file - To write cleaned results

# Tesseract Command Format

tesseract <input_image> <output_base> [options]

Common options:
- -l <lang> - Language (eng, spa, fra, deu, etc.)
- --psm <n> - Page segmentation mode:
  0 = Orientation and script detection (OSD) only
  1 = Automatic page segmentation with OSD
  3 = Fully automatic page segmentation (default)
  6 = Assume a single uniform block of text
  11 = Sparse text. Find as much text as possible
- --oem <n> - OCR Engine mode:
  0 = Legacy engine
  1 = Neural nets LSTM engine
  2 = Legacy + LSTM engines
  3 = Default (based on what is available)

# Workflow

1. Run tesseract command to extract text from image
2. Read the output file (tesseract adds .txt automatically)
3. Analyze the extracted text
4. If requested, clean and format the text:
   - Fix OCR errors (common misrecognitions like l/I, O/0, etc.)
   - Improve formatting and structure
   - Remove noise/artifacts
   - Preserve intended structure (paragraphs, lists, tables)
5. Write the final result to output file if requested

# Example Workflow

User: "Extract text from invoice.png"

1. Run: tesseract invoice.png output -l eng --psm 6
2. Read: output.txt
3. Present the extracted text to user

User: "Extract and clean text from document.png"

1. Run: tesseract document.png output -l eng
2. Read: output.txt
3. Analyze the text and clean it:
   - Fix common OCR mistakes
   - Improve formatting
   - Structure paragraphs properly
4. Present cleaned text to user

# Important Notes

- You can ONLY run tesseract command (no other bash commands will work)
- Tesseract automatically adds .txt extension to output files
- For PDF files, you may need to specify page ranges
- If OCR quality is poor, suggest trying different --psm values
- Always validate that input files exist before running tesseract
"#.to_string()
}

/// Helper function to extract text from content blocks
fn extract_text_from_content(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        // Note: This test requires setting up mock dependencies
        // Skipping for now due to complexity of mocking LlmClient and Database
    }

    #[test]
    fn test_system_prompt_generation() {
        let prompt = generate_system_prompt();
        assert!(prompt.contains("tesseract"));
        assert!(prompt.contains("OCR"));
        assert!(prompt.contains("--psm"));
    }
}
```

#### 1.3 Export Module

**File**: `nocodo-agents/src/lib.rs`

Add module declaration:

```rust
pub mod codebase_analysis;
pub mod database;
pub mod factory;
pub mod sqlite_analysis;
pub mod tesseract;  // ← ADD THIS
pub mod tools;
```

### Phase 2: Update Agent Factory

#### 2.1 Add TesseractAgent to Factory

**File**: `nocodo-agents/src/factory.rs`

Add import and factory method:

```rust
use crate::tesseract::TesseractAgent;

impl AgentFactory {
    // ... existing methods

    /// Create a TesseractAgent for OCR tasks
    ///
    /// # Arguments
    /// * `base_path` - Working directory for file operations
    ///
    /// # Examples
    /// ```rust
    /// let factory = AgentFactory::new(/* config */)?;
    /// let agent = factory.create_tesseract_agent(PathBuf::from("/path/to/images"))?;
    /// ```
    pub fn create_tesseract_agent(
        &self,
        base_path: PathBuf,
    ) -> anyhow::Result<TesseractAgent> {
        TesseractAgent::new(
            self.llm_client.clone(),
            self.database.clone(),
            base_path,
        )
    }
}
```

### Phase 3: Add Pre-condition Verification

#### 3.1 Create Tesseract Verification Utility

**File**: `nocodo-agents/src/tesseract/mod.rs`

Add helper function to check tesseract installation:

```rust
/// Verify that Tesseract is installed and accessible
pub fn verify_tesseract_installation() -> anyhow::Result<String> {
    use std::process::Command;

    let output = Command::new("tesseract")
        .arg("--version")
        .output()
        .context("Failed to execute 'tesseract --version'. Is Tesseract installed?")?;

    if !output.status.success() {
        anyhow::bail!(
            "Tesseract command failed. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let version_info = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(version_info)
}

impl TesseractAgent {
    /// Verify pre-conditions before creating agent
    pub fn verify_preconditions() -> anyhow::Result<()> {
        match verify_tesseract_installation() {
            Ok(version) => {
                tracing::info!("Tesseract OCR found: {}", version.lines().next().unwrap_or(""));
                Ok(())
            }
            Err(e) => {
                anyhow::bail!(
                    "Tesseract OCR is not installed or not accessible.\n\
                     Error: {}\n\
                     \n\
                     Installation instructions:\n\
                     - macOS: brew install tesseract\n\
                     - Ubuntu/Debian: sudo apt-get install tesseract-ocr\n\
                     - Windows: Download from https://github.com/UB-Mannheim/tesseract/wiki\n\
                     \n\
                     After installation, verify with: tesseract --version",
                    e
                )
            }
        }
    }
}
```

Update the `new()` method to optionally verify:

```rust
impl TesseractAgent {
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        base_path: PathBuf,
    ) -> anyhow::Result<Self> {
        // Optional: Verify tesseract installation
        // Uncomment to make it mandatory:
        // Self::verify_preconditions()?;

        // ... rest of implementation
    }
}
```

## Files Changed

### New Files
- `nocodo-agents/src/tesseract/mod.rs` - TesseractAgent implementation
- `nocodo-agents/tasks/add-tesseract-ocr-agent.md` - This task document

### Modified Files
- `nocodo-agents/src/lib.rs` - Export tesseract module
- `nocodo-agents/src/factory.rs` - Add factory method for TesseractAgent

## Build & Quality Checks

### Compilation
```bash
cd nocodo-agents
cargo build
```

### Code Quality
```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

### Type Check
```bash
cargo check
```

## Usage Examples

### Basic OCR Extraction

```rust
use nocodo_agents::{Agent, factory::AgentFactory};
use std::path::PathBuf;

let factory = AgentFactory::new(/* config */)?;

// Create agent with working directory
let agent = factory.create_tesseract_agent(
    PathBuf::from("/path/to/images")
)?;

// Extract text from image
let result = agent.execute(
    "Extract text from document.png",
    session_id
).await?;

println!("Extracted text:\n{}", result);
```

### OCR with Cleaning

```rust
let result = agent.execute(
    "Extract text from receipt.jpg and clean it up. Fix any OCR errors and format it nicely.",
    session_id
).await?;
```

### OCR with Language Specification

```rust
let result = agent.execute(
    "Extract French text from french_document.png using -l fra",
    session_id
).await?;
```

### OCR from PDF

```rust
let result = agent.execute(
    "Extract text from scanned_document.pdf and save the cleaned version to output.txt",
    session_id
).await?;
```

## Testing Strategy

### Manual Testing

Since this agent relies on external dependencies (Tesseract) and multi-turn LLM interactions, manual testing is the primary approach:

1. **Installation verification**:
   ```bash
   tesseract --version
   ```

2. **Basic OCR**:
   - Create test image with text
   - Run agent with simple extraction request
   - Verify output matches image content

3. **OCR with cleaning**:
   - Use image with OCR challenges (poor quality, noise)
   - Request extraction + cleaning
   - Verify LLM improves output quality

4. **Different formats**:
   - Test with PNG, JPG, TIFF, PDF
   - Verify all formats work

5. **Command restriction**:
   - Verify agent can run tesseract
   - Verify agent CANNOT run other commands (ls, cat, rm, etc.)
   - Check error messages for denied commands

### Test Images

Create test images with known content:
- Simple text: "Hello World"
- Multi-line text with paragraphs
- Text with special characters
- Poor quality scan simulation

### Security Testing

```rust
// Verify bash restriction
let result = agent.execute(
    "Run ls -la to see files",
    session_id
).await;

// Should fail with permission error
assert!(result.is_err());
```

## Security Considerations

### Bash Command Restrictions

The agent uses `BashPermissions::minimal(vec!["tesseract"])`:
- ✅ Allows: `tesseract input.png output`
- ❌ Denies: `ls -la`
- ❌ Denies: `cat file.txt`
- ❌ Denies: `rm file.txt`
- ❌ Denies: `curl`, `wget`, `git`, etc.

### File Access Restrictions

- All file operations limited to `base_path`
- Tool executor enforces path restrictions
- Cannot access files outside working directory

### Input Validation

- Tesseract validates image file formats
- File paths validated by tool executor
- Command injection prevented by bash permissions

## Success Criteria

- [ ] TesseractAgent module created with full implementation
- [ ] Agent uses restricted bash (only tesseract command)
- [ ] Tools include Bash, ReadFile, WriteFile
- [ ] System prompt includes tesseract usage instructions
- [ ] Pre-conditions check for tesseract installation
- [ ] Factory method added for creating TesseractAgent
- [ ] Module exported in lib.rs
- [ ] Code compiles without errors
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Manual testing passes:
  - [ ] Can extract text from images
  - [ ] Can clean OCR output with LLM
  - [ ] Cannot execute other bash commands
  - [ ] Works with different image formats

## Future Enhancements

### Phase 2: Advanced Features

1. **Batch processing**: Process multiple images in one request
2. **Language detection**: Auto-detect language and use appropriate -l flag
3. **Quality assessment**: Evaluate OCR confidence and suggest improvements
4. **Format preservation**: Better handling of tables, columns, complex layouts
5. **PDF page handling**: Extract specific pages from multi-page PDFs

### Phase 3: Integration Features

1. **Image preprocessing**: Integration with ImageMagick for image enhancement
2. **Post-processing pipeline**: Chain multiple cleaning/formatting steps
3. **Output formats**: Support markdown, JSON, structured data extraction
4. **Comparison mode**: Compare OCR output from different PSM/OEM settings

## Notes

- Testing deferred due to complexity of mocking multi-turn tool calling and external dependencies
- Agent follows same pattern as SqliteAnalysisAgent and CodebaseAnalysisAgent for consistency
- Tesseract must be installed on system - agent will provide helpful error messages if missing
- The agent is secure by design - restricted bash access and file path validation
- LLM cleaning is a key differentiator - not just running tesseract, but intelligently processing output

## References

- **Tesseract documentation**: https://tesseract-ocr.github.io/tessdoc/
- **Tesseract GitHub**: https://github.com/tesseract-ocr/tesseract
- **Tesseract command line usage**: https://tesseract-ocr.github.io/tessdoc/Command-Line-Usage.html
- **Page segmentation modes**: https://tesseract-ocr.github.io/tessdoc/ImproveQuality.html#page-segmentation-method
