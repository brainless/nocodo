# Nocodo Manager LLM Agent Architecture Analysis

**Analysis Date**: November 10, 2025  
**Directory**: `/home/brainless/Projects/nocodo`  
**Focus**: LLM Agent Implementation, Tool System, and Integration Architecture

---

## Executive Summary

The nocodo manager implements a sophisticated LLM agent architecture with:

1. **Three-layer adapter pattern** for multi-provider LLM support
2. **Native tool calling** for file operations, code search, and patching
3. **Conversation state management** with database persistence
4. **Provider-agnostic unified client** supporting Claude, OpenAI, xAI/Grok, and zAI/GLM
5. **Complete tool execution pipeline** with streaming via WebSocket

---

## 1. LLM Agent Implementation

### 1.1 Core Agent Structure

**File**: `/home/brainless/Projects/nocodo/manager/src/llm_agent.rs`

```rust
pub struct LlmAgent {
    db: Arc<Database>,                    // Persistent conversation storage
    ws: Arc<WebSocketBroadcaster>,       // Real-time tool call updates
    tool_executor: ToolExecutor,         // File operation handler
    config: Arc<AppConfig>,              // Runtime configuration
}
```

### 1.2 Main Agent Flows

#### A. Session Creation
```rust
pub async fn create_session(
    &self,
    work_id: i64,
    provider: String,
    model: String,
    system_prompt: Option<String>,
) -> Result<LlmAgentSession>
```

**Flow**:
1. Create `LlmAgentSession` record in database
2. Store system prompt as initial message (if provided)
3. Return session with auto-generated ID

**Data Model** (`/home/brainless/Projects/nocodo/manager/src/models.rs:627`):
```rust
pub struct LlmAgentSession {
    pub id: i64,                          // Database ID
    pub work_id: i64,                     // Associated work/project
    pub provider: String,                 // "anthropic", "openai", "xai", "zai"
    pub model: String,                    // Model ID (e.g., "claude-sonnet-4-5")
    pub status: String,                   // "running" | "failed"
    pub system_prompt: Option<String>,    // Initial instructions
    pub started_at: i64,                  // Unix timestamp
    pub ended_at: Option<i64>,            // Completion time
}
```

#### B. Message Processing Loop
```rust
pub async fn process_message(
    &self,
    session_id: i64,
    user_message: String,
) -> Result<String>
```

**Execution Flow**:

1. **Store User Message** (lines 104-110)
   - Insert message with role="user" into database
   - Retrieve full conversation history (all messages for session)

2. **Create LLM Client** (lines 120-146)
   ```rust
   let config = LlmProviderConfig {
       provider: session.provider.clone(),
       model: session.model.clone(),
       api_key: self.get_api_key(&session.provider)?,
       base_url: self.get_base_url(&session.provider),
       max_tokens: Some(4000),
       temperature: Some(0.7),  // Special handling for zAI (omitted)
   };
   let llm_client = create_llm_client(config)?;
   ```

3. **Reconstruct Conversation** (lines 150-202)
   - Parse stored message history
   - Extract tool calls from previous assistant responses (stored as JSON)
   - Handle tool results from conversation history
   - Build unified message list for LLM

4. **Create Tool Definitions** (line 233)
   - Generate JSON Schema for available tools
   - Controlled via `ENABLE_TOOLS` environment variable
   - Options: "none", "list_files", "list_read", "all" (default)

5. **Send Request to LLM** (lines 243-263)
   ```rust
   let request = LlmCompletionRequest {
       model: session.model.clone(),
       messages,
       max_tokens: Some(4000),
       temperature,
       stream: Some(false),
       tools: Some(self.create_native_tool_definitions()),
       tool_choice: Some(ToolChoice::Auto("auto".to_string())),
       // ... other fields
   };
   let response = llm_client.complete(request).await?;
   ```

6. **Process LLM Response** (lines 265-330)
   - Extract text content
   - Clean response (remove unwanted prefixes)
   - Extract tool calls from response
   - Log all response structure details

7. **Broadcast Assistant Response** (lines 332-334)
   - Send text response via WebSocket immediately
   - Allows UI to show response while tools execute

8. **Store Response** (lines 344-366)
   - Store as structured JSON with tool calls
   - Format: `{"text": "...", "tool_calls": [...]}`
   - Enables proper reconstruction in future turns

9. **Execute Tool Calls** (lines 368-400)
   - Call `process_native_tool_calls()` if any tools were invoked
   - Process sequentially with error handling
   - Update tool call records with results

---

## 2. Tool System Architecture

### 2.1 Tool Definition & Registration

**File**: `/home/brainless/Projects/nocodo/manager/src/llm_agent.rs:1273`

```rust
fn create_native_tool_definitions(&self) -> Vec<ToolDefinition>
```

**Five Available Tools**:

| Tool | Purpose | Status |
|------|---------|--------|
| `list_files` | List directory contents with metadata | Production |
| `read_file` | Read file contents with size limits | Production |
| `write_file` | Write/create/append files | Production |
| `grep` | Search files with regex patterns | Production |
| `apply_patch` | Unified diff application for bulk changes | Production |

**Tool Definition Structure** (`/home/brainless/Projects/nocodo/manager/src/llm_client.rs:291`):
```rust
pub struct ToolDefinition {
    pub r#type: String,                    // Always "function"
    pub function: FunctionDefinition,
}

pub struct FunctionDefinition {
    pub name: String,                      // Tool name
    pub description: String,               // Usage description
    pub parameters: serde_json::Value,    // JSON Schema
}
```

### 2.2 Tool Execution Pipeline

**File**: `/home/brainless/Projects/nocodo/manager/src/llm_agent.rs:429-700`

```rust
async fn process_native_tool_calls(
    &self,
    session_id: i64,
    tool_calls: &[LlmToolCall],
) -> Result<()>
```

**Per-Tool Execution Flow** (lines 443-676):

1. **Parse Tool Call** (lines 453-561)
   - Match function name to request type
   - Deserialize JSON arguments into typed struct
   - Error handling with detailed logging

2. **Create Tool Call Record** (lines 570-614)
   ```rust
   let mut tool_call_record = LlmAgentToolCall::new(
       session_id,
       tool_name,
       serde_json::to_value(&tool_request)?,
   );
   tool_call_record.status = "executing";
   let tool_call_id = self.db.create_llm_agent_tool_call(&tool_call_record)?;
   ```

3. **Broadcast Tool Start** (lines 607-614)
   - Send WebSocket message: `tool_call_started`
   - Includes tool ID, name, and session ID
   - UI can show spinner or progress

4. **Execute Tool** (lines 624-626)
   ```rust
   let project_tool_executor = self.get_tool_executor_for_session(session_id).await?;
   let tool_response = project_tool_executor.execute(tool_request).await;
   ```

5. **Handle Result** (lines 629-676)
   - **Success**: Mark tool call complete, broadcast response
   - **Error**: Mark tool call failed, broadcast error

6. **Add Result to Conversation** (lines 685-699)
   - Store tool result as message with role="tool"
   - Provider-specific formatting:
     - **Claude**: Use `tool_use_id` format
     - **OpenAI-compatible**: Use `tool_call_id` format

### 2.3 Tool Executor Implementation

**File**: `/home/brainless/Projects/nocodo/manager/src/tools.rs`

```rust
pub struct ToolExecutor {
    base_path: PathBuf,          // Project root
    max_file_size: u64,          // Default: 1MB
}

pub async fn execute(&self, request: ToolRequest) -> Result<ToolResponse>
```

**Request-Response Types** (`/home/brainless/Projects/nocodo/manager/src/models.rs:200-600`):

```rust
pub enum ToolRequest {
    ListFiles(ListFilesRequest),
    ReadFile(ReadFileRequest),
    WriteFile(WriteFileRequest),
    Grep(GrepRequest),
    ApplyPatch(ApplyPatchRequest),
}

pub enum ToolResponse {
    ListFiles(ListFilesResponse),
    ReadFile(ReadFileResponse),
    WriteFile(WriteFileResponse),
    Grep(GrepResponse),
    ApplyPatch(ApplyPatchResponse),
    Error(ToolErrorResponse),
}
```

**Tool Error Handling**:
```rust
pub enum ToolError {
    FileNotFound(String),
    PermissionDenied(String),
    InvalidPath(String),
    FileTooLarge(u64, u64),     // bytes, max_bytes
    IoError(String),
    SerializationError(String),
}
```

---

## 3. LLM Client Architecture (Adapter Pattern)

### 3.1 Three-Layer Architecture

```
┌─────────────────────────────────────────────┐
│  Application Layer: LlmAgent                │
│  - Session management                       │
│  - Tool orchestration                       │
│  - Conversation persistence                 │
└────────────┬────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────┐
│  Unified Client Layer: UnifiedLlmClient    │
│  - Provider-agnostic interface              │
│  - Request/response translation             │
│  - Tool call extraction                     │
└────────────┬────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────┐
│  Adapter Layer: ProviderAdapter Trait       │
│  - Provider-specific request formatting     │
│  - HTTP communication                       │
│  - Response parsing and mapping             │
└────────────┬────────────────────────────────┘
             │
    ┌────────┼────────┬─────────┬─────────┐
    ▼        ▼        ▼         ▼         ▼
  Claude  OpenAI   Responses  zAI/GLM  xAI/Grok
  (Claude (Chat    API        (Chat    (Chat
   Messages) Completions) (gpt-5)  Completions) Completions)
```

### 3.2 Core Trait Definitions

**File**: `/home/brainless/Projects/nocodo/manager/src/llm_client/adapters/trait_adapter.rs`

```rust
pub trait ProviderRequest: Send + Sync {
    fn to_json(&self) -> Result<Value>;
    fn custom_headers(&self) -> Vec<(String, String)> { vec![] }
}

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn get_api_url(&self) -> String;
    fn supports_native_tools(&self) -> bool;
    fn prepare_request(&self, request: LlmCompletionRequest) -> Result<Box<dyn ProviderRequest>>;
    async fn send_request(&self, request: Box<dyn ProviderRequest>) -> Result<reqwest::Response>;
    fn parse_response(&self, response_text: &str) -> Result<LlmCompletionResponse>;
    fn extract_tool_calls(&self, response: &LlmCompletionResponse) -> Vec<LlmToolCall>;
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
}
```

### 3.3 Unified Client Implementation

**File**: `/home/brainless/Projects/nocodo/manager/src/llm_client/unified_client.rs`

```rust
pub struct UnifiedLlmClient {
    adapter: Box<dyn ProviderAdapter>,
    config: LlmProviderConfig,
}

#[async_trait]
impl LlmClient for UnifiedLlmClient {
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        // 1. Prepare provider-specific request
        let provider_request = self.adapter.prepare_request(request)?;
        
        // 2. Send HTTP request
        let response = self.adapter.send_request(provider_request).await?;
        
        // 3. Check status and extract text
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API error: {} - {}", status, error_text));
        }
        
        // 4. Parse provider-specific response to unified format
        let response_text = response.text().await?;
        let llm_response = self.adapter.parse_response(&response_text)?;
        
        Ok(llm_response)
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        self.adapter.extract_tool_calls(response)
    }
}
```

### 3.4 Unified Data Types

**File**: `/home/brainless/Projects/nocodo/manager/src/llm_client.rs:131-265`

```rust
pub struct LlmMessage {
    pub role: String,                      // "system", "user", "assistant", "tool"
    pub content: Option<String>,
    pub tool_calls: Option<Vec<LlmToolCall>>,
    pub function_call: Option<LlmFunctionCall>,  // Legacy OpenAI
    pub tool_call_id: Option<String>,     // For tool responses
}

pub struct LlmCompletionRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub tool_choice: Option<ToolChoice>,
    pub functions: Option<Vec<FunctionDefinition>>,  // Legacy
    pub function_call: Option<FunctionCall>,         // Legacy
}

pub struct LlmCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<LlmChoice>,
    pub usage: Option<LlmUsage>,
}

pub struct LlmToolCall {
    pub id: String,
    pub r#type: String,                    // "function"
    pub function: LlmToolCallFunction,
}

pub struct LlmToolCallFunction {
    pub name: String,                      // Tool name
    pub arguments: String,                 // JSON string of args
}
```

---

## 4. Provider-Specific Implementations

### 4.1 Factory Method

**File**: `/home/brainless/Projects/nocodo/manager/src/llm_client.rs:2065`

```rust
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    match (config.provider.to_lowercase().as_str(), config.model.as_str()) {
        // Claude 4.5/4.1
        ("anthropic" | "claude", model) if is_claude_45_or_41(model) => {
            let adapter = Box::new(adapters::ClaudeMessagesAdapter::new(config.clone())?);
            Ok(Box::new(UnifiedLlmClient::new(adapter, config)?))
        }

        // GPT-5 Responses API (gpt-5-codex)
        ("openai", "gpt-5") | ("openai", "gpt-5-codex") => {
            let adapter = Box::new(adapters::ResponsesApiAdapter::new(config.clone())?);
            Ok(Box::new(UnifiedLlmClient::new(adapter, config)?))
        }

        // Other OpenAI models (GPT-4, etc.)
        ("openai", _) => {
            let client = OpenAiCompatibleClient::new(config)?;
            Ok(Box::new(client))
        }

        // xAI Grok
        ("grok" | "xai", _) => {
            let client = OpenAiCompatibleClient::new(config)?;
            Ok(Box::new(client))
        }

        // zAI GLM-4
        ("zai" | "glm", _) => {
            let adapter = Box::new(adapters::GlmChatCompletionsAdapter::new(config.clone())?);
            Ok(Box::new(UnifiedLlmClient::new(adapter, config)?))
        }

        _ => Err(anyhow::anyhow!("Unsupported LLM provider: {}", config.provider)),
    }
}
```

### 4.2 Existing Adapter Implementations

#### A. Claude Messages Adapter
**File**: `/home/brainless/Projects/nocodo/manager/src/llm_client/adapters/claude_messages.rs`

**Provider Type**: Anthropic Messages API  
**Tool Format**: Content blocks with type="tool_use"  
**Supported Models**: claude-sonnet-4-5-20250929, claude-haiku-4-5-20251001, claude-opus-4-1-20250805

**Special Handling**:
- System message passed as separate field
- Tool responses use content blocks format
- Tool use ID in responses

#### B. GLM Chat Completions Adapter
**File**: `/home/brainless/Projects/nocodo/manager/src/llm_client/adapters/glm_chat_completions.rs`

**Provider Type**: zAI GLM-4 Chat Completions  
**Tool Format**: OpenAI-compatible format  
**Supported Models**: GLM-4, GLM-4V, etc.

**Special Handling**:
- Temperature parameter omitted to avoid floating-point precision issues
- Empty tools array must be null (not empty array)
- Tool choice specification

#### C. Responses API Adapter
**File**: `/home/brainless/Projects/nocodo/manager/src/llm_client/adapters/responses_api.rs`

**Provider Type**: OpenAI Responses API (gpt-5-codex)  
**Tool Format**: Function call output items  
**Supported Models**: gpt-5-codex

**Special Handling**:
- Different endpoint: `/v1/responses` instead of `/v1/chat/completions`
- Output items include Message, Reasoning, FunctionCall types
- Instructions field for system messages
- Codex-specific system instructions

#### D. OpenAI Compatible Client
**File**: `/home/brainless/Projects/nocodo/manager/src/llm_client.rs:384-1120`

**Provider Type**: OpenAI Chat Completions (not adapter)  
**Tool Format**: OpenAI native tools + legacy function calls  
**Supported Providers**: OpenAI (GPT-4), xAI/Grok

**Special Handling**:
- Tool calling capability detection
- Legacy function call support
- Model-based capability checking

---

## 5. Tool Call Extraction and Processing

### 5.1 Tool Call Extraction Pattern

**OpenAI Format** (tool_calls in message):
```json
{
  "message": {
    "role": "assistant",
    "tool_calls": [{
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "list_files",
        "arguments": "{\"path\": \"/home\"}"
      }
    }]
  }
}
```

**Claude Format** (tool_use content blocks):
```json
{
  "content": [
    {
      "type": "text",
      "text": "I'll list the files for you."
    },
    {
      "type": "tool_use",
      "id": "toolu_abc123",
      "name": "list_files",
      "input": {"path": "/home"}
    }
  ]
}
```

**Responses API Format** (FunctionCall items):
```json
{
  "output": [
    {
      "type": "function_call",
      "id": "call_abc123",
      "name": "list_files",
      "arguments": "{\"path\": \"/home\"}"
    }
  ]
}
```

### 5.2 Extraction Implementation

**File**: `/home/brainless/Projects/nocodo/manager/src/llm_client.rs:475-542`

```rust
fn extract_tool_calls_from_response_internal(
    &self,
    response: &LlmCompletionResponse,
) -> Vec<LlmToolCall> {
    let mut tool_calls = Vec::new();
    
    for choice in &response.choices {
        // Check message-level tool calls (OpenAI format)
        if let Some(message) = &choice.message {
            if let Some(message_tool_calls) = &message.tool_calls {
                tool_calls.extend(message_tool_calls.clone());
            }
            
            // Check legacy function calls (older OpenAI)
            if let Some(function_call) = &message.function_call {
                tool_calls.push(LlmToolCall {
                    id: format!("legacy-{}", Uuid::new_v4()),
                    r#type: "function".to_string(),
                    function: LlmToolCallFunction {
                        name: function_call.name.clone(),
                        arguments: function_call.arguments.clone(),
                    },
                });
            }
        }
        
        // Check choice-level tool calls (Anthropic format)
        if let Some(choice_tool_calls) = &choice.tool_calls {
            tool_calls.extend(choice_tool_calls.clone());
        }
    }
    
    tool_calls
}
```

---

## 6. Conversation State Management

### 6.1 Message Storage and Reconstruction

**Database Tables**:
- `llm_agent_sessions` - Session metadata
- `llm_agent_messages` - Conversation history
- `llm_agent_tool_calls` - Tool execution records

**Message Structure** (`/home/brainless/Projects/nocodo/manager/src/models.rs:669`):
```rust
pub struct LlmAgentMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,           // "user", "assistant", "system", "tool"
    pub content: String,        // Raw text or JSON with metadata
    pub created_at: i64,
}
```

### 6.2 Assistant Response Encoding

**Lines 343-354** of `llm_agent.rs`:

Assistant responses with tool calls are stored as JSON:
```json
{
  "text": "Let me search for that file.",
  "tool_calls": [
    {
      "id": "call_123",
      "type": "function",
      "function": {
        "name": "grep",
        "arguments": "{\"pattern\": \"error\", \"path\": \".\"}"
      }
    }
  ]
}
```

**Plain responses** (no tool calls):
```
Just plain text content
```

### 6.3 Message Reconstruction

**Lines 150-202** of `llm_agent.rs`:

```rust
for msg in &history {
    // Parse assistant messages
    let (content, tool_calls) = if msg.role == "assistant" {
        if let Ok(assistant_data) = serde_json::from_str::<Value>(&msg.content) {
            // Extract text and tool_calls from JSON
            let text = assistant_data.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let tool_calls = assistant_data.get("tool_calls").and_then(|v| v.as_array())
                .and_then(|calls| {
                    let parsed: Vec<LlmToolCall> = calls.iter()
                        .filter_map(|tc| serde_json::from_value(tc.clone()).ok())
                        .collect();
                    if parsed.is_empty() { None } else { Some(parsed) }
                });
            (Some(text.to_string()), tool_calls)
        } else {
            // Plain text response
            (Some(msg.content.clone()), None)
        }
    } else {
        // Other messages use content as-is
        (Some(msg.content.clone()), None)
    };
    
    messages.push(LlmMessage {
        role: msg.role.clone(),
        content,
        tool_calls,
        // ...
    });
}
```

---

## 7. Configuration and Integration

### 7.1 Provider Configuration

**File**: `/home/brainless/Projects/nocodo/manager/src/models.rs:616`

```rust
pub struct LlmProviderConfig {
    pub provider: String,              // "anthropic", "openai", "xai", "zai"
    pub model: String,                 // Model ID
    pub api_key: String,               // API authentication
    pub base_url: Option<String>,      // Custom endpoint (for self-hosted)
    pub max_tokens: Option<u32>,       // Output token limit
    pub temperature: Option<f32>,      // Sampling temperature
}
```

### 7.2 API Key Retrieval

**Lines 120-135** of `llm_agent.rs`:

```rust
let config = LlmProviderConfig {
    provider: session.provider.clone(),
    model: session.model.clone(),
    api_key: self.get_api_key(&session.provider)?,
    base_url: self.get_base_url(&session.provider),
    max_tokens: Some(4000),
    temperature: if session.provider.to_lowercase() == "zai" { 
        None  // zAI omits temperature
    } else { 
        Some(0.3) 
    },
};
```

**Methods**:
- `get_api_key(provider: &str) -> Result<String>` - Retrieves from config
- `get_base_url(provider: &str) -> Option<String>` - Optional custom URL

---

## 8. WebSocket Broadcasting

### 8.1 Real-time Tool Updates

**File**: `/home/brainless/Projects/nocodo/manager/src/websocket.rs`

**Events Broadcasted** (from `llm_agent.rs`):

1. **Agent Response Chunk** (line 333):
   ```rust
   self.ws.broadcast_llm_agent_chunk(session_id, response_text).await;
   ```

2. **Tool Call Started** (lines 608-614):
   ```rust
   self.ws.broadcast_tool_call_started(
       session_id,
       tool_call_id.to_string(),
       tool_name.to_string(),
   ).await;
   ```

3. **Tool Call Completed** (lines 641-647):
   ```rust
   self.ws.broadcast_tool_call_completed(
       session_id,
       tool_call_id.to_string(),
       response_json.clone(),
   ).await;
   ```

4. **Tool Call Failed** (lines 666-672):
   ```rust
   self.ws.broadcast_tool_call_failed(
       session_id,
       tool_call_id.to_string(),
       error_message,
   ).await;
   ```

---

## 9. Error Handling Strategy

### 9.1 Tool Execution Error Handling

**Lines 629-676** of `llm_agent.rs`:

```rust
let tool_response = project_tool_executor.execute(tool_request).await;

match tool_response {
    Ok(response) => {
        // Mark as complete, store response
        tool_call_record.complete(serde_json::to_value(&response)?);
        // Broadcast success
        self.ws.broadcast_tool_call_completed(...).await;
    }
    Err(e) => {
        // Mark as failed, store error
        tool_call_record.fail(e.to_string());
        // Broadcast failure with error message
        self.ws.broadcast_tool_call_failed(...).await;
    }
}
self.db.update_llm_agent_tool_call(&tool_call_record)?;
```

### 9.2 Tool Call Parsing Errors

**Lines 453-561** of `llm_agent.rs`:

```rust
let tool_request = match tool_call.function.name.as_str() {
    "list_files" => {
        match serde_json::from_str::<ListFilesRequest>(&tool_call.function.arguments) {
            Ok(request) => ToolRequest::ListFiles(request),
            Err(e) => {
                tracing::error!(
                    session_id = %session_id,
                    error = %e,
                    arguments = %tool_call.function.arguments,
                    "Failed to parse list_files arguments"
                );
                continue;  // Skip this tool call
            }
        }
    }
    // ... other tools ...
    unknown_function => {
        tracing::error!(
            function_name = %unknown_function,
            "Unknown function name in tool call"
        );
        continue;
    }
};
```

---

## 10. Key Design Patterns

### 10.1 Adapter Pattern

**Purpose**: Support multiple LLM providers with different API formats

**Components**:
- **Trait**: `ProviderAdapter` - Unified interface
- **Implementations**: Claude, OpenAI Responses, GLM adapters
- **Unified Client**: `UnifiedLlmClient` wraps any adapter

**Benefit**: Add new providers by implementing adapter only

### 10.2 Tool Choice Progressive Testing

**Environment Variable**: `ENABLE_TOOLS`

**Options**:
- `"none"` - No tools (basic chat testing)
- `"list_files"` - Only list_files
- `"list_read"` - list_files + read_file
- `"all"` (default) - All five tools

**Lines 1283-1290** of `llm_agent.rs`:
```rust
let enable_tools = std::env::var("ENABLE_TOOLS").unwrap_or_else(|_| "all".to_string());
match enable_tools.as_str() {
    "none" => vec![],
    "list_files" => vec![/* list_files only */],
    "list_read" => vec![/* list_files, read_file */],
    _ => vec![/* all tools */],
}
```

### 10.3 Structured Response Storage

**Pattern**: Store assistant responses as JSON when tool calls present

**Benefits**:
- Reconstruction of tool calls in future turns
- Consistent UI display across providers
- Enables analysis of agent behavior

---

## 11. Process Execution Capabilities

### 11.1 Current Tool Implementation

The system provides **file operation tools** rather than shell execution:

1. **list_files** - Directory traversal
2. **read_file** - File content reading
3. **write_file** - File creation/modification
4. **grep** - Pattern searching
5. **apply_patch** - Bulk file changes via unified diff

### 11.2 Command Execution Capability

**Current Status**: No direct shell execution tool  
**Reason**: Security and process management complexity  
**Alternative**: Agents operate on files and can request user approval for shell commands

### 11.3 Tool Execution Context

**Project-Specific Isolation** (lines 412-427):
```rust
async fn get_tool_executor_for_session(&self, session_id: i64) -> Result<ToolExecutor> {
    let session = self.db.get_llm_agent_session(session_id)?;
    let work = self.db.get_work_by_id(session.work_id)?;
    
    if let Some(project_id) = work.project_id {
        let project = self.db.get_project_by_id(project_id)?;
        Ok(ToolExecutor::new(PathBuf::from(project.path)))  // Project-specific root
    } else {
        Ok(ToolExecutor::new(self.tool_executor.base_path().clone()))
    }
}
```

**Path Validation**: All paths validated against base_path to prevent escape

---

## 12. File Structure Summary

```
manager/src/
├── llm_agent.rs                          # Main agent orchestration
│   ├── LlmAgent struct
│   ├── create_session()
│   ├── process_message()
│   ├── process_native_tool_calls()
│   ├── create_native_tool_definitions()
│   └── Tool call parsing & execution
│
├── llm_client.rs                         # LLM client factory and traits
│   ├── LlmClient trait (async methods)
│   ├── LlmCompletionRequest struct
│   ├── LlmCompletionResponse struct
│   ├── LlmToolCall struct
│   ├── ToolDefinition struct
│   ├── OpenAiCompatibleClient
│   └── create_llm_client() factory
│
├── llm_client/
│   ├── adapters/
│   │   ├── trait_adapter.rs              # ProviderAdapter trait
│   │   ├── claude_messages.rs            # Claude implementation
│   │   ├── glm_chat_completions.rs       # zAI/GLM implementation
│   │   ├── responses_api.rs              # GPT-5 Responses API
│   │   └── mod.rs                        # Re-exports
│   │
│   ├── types/
│   │   ├── claude_types.rs               # Claude request/response types
│   │   ├── glm_types.rs                  # GLM request/response types
│   │   ├── responses_types.rs            # Responses API types
│   │   └── mod.rs                        # Re-exports
│   │
│   └── unified_client.rs                 # UnifiedLlmClient wrapper
│
├── llm_providers/
│   ├── anthropic.rs                      # AnthropicProvider
│   ├── openai.rs                         # OpenAiProvider
│   ├── xai.rs                            # XaiProvider
│   ├── zai.rs                            # ZaiProvider
│   └── mod.rs                            # Re-exports
│
├── tools.rs                              # ToolExecutor implementation
│   ├── list_files()
│   ├── read_file()
│   ├── write_file()
│   ├── grep_search()
│   └── apply_patch()
│
├── models.rs                             # Data structures
│   ├── LlmProviderConfig
│   ├── LlmAgentSession
│   ├── LlmAgentMessage
│   ├── Tool request/response types
│   └── Error types
│
├── main.rs                               # Server initialization
├── handlers.rs                           # HTTP API endpoints
├── database.rs                           # Database layer
├── websocket.rs                          # WebSocket broadcaster
└── config.rs                             # Configuration loading
```

---

## 13. Integration Points

### 13.1 HTTP API Entry Points

**File**: `/home/brainless/Projects/nocodo/manager/src/handlers.rs`

- **POST /api/v1/llm-agent-session** - Create session
- **POST /api/v1/llm-agent-message** - Process message
- **GET /api/v1/llm-agent-session/:id** - Get session
- **GET /api/v1/llm-agent-message/:id** - Get message

### 13.2 Database Integration

**File**: `/home/brainless/Projects/nocodo/manager/src/database.rs`

Methods:
- `create_llm_agent_session()`
- `get_llm_agent_session()`
- `create_llm_agent_message()`
- `get_llm_agent_messages()`
- `create_llm_agent_tool_call()`
- `update_llm_agent_tool_call()`

### 13.3 WebSocket Events

**Broadcasted to connected clients**:
- `llm_agent_chunk` - Response text chunks
- `tool_call_started` - Tool execution beginning
- `tool_call_completed` - Tool execution succeeded
- `tool_call_failed` - Tool execution error

---

## 14. Testing Infrastructure

### 14.1 Test Files

Located in `/home/brainless/Projects/nocodo/manager/tests/`:

- `llm_e2e_real_test.rs` - End-to-end integration tests
- `integration/llm_agent.rs` - Agent-specific tests
- `common/llm_config.rs` - Test configuration helpers

### 14.2 E2E Test Script

**File**: `run_llm_e2e_test.sh`

**Usage**:
```bash
./run_llm_e2e_test.sh <provider> <model>
./run_llm_e2e_test.sh anthropic claude-sonnet-4-5
./run_llm_e2e_test.sh openai gpt-4
./run_llm_e2e_test.sh zai glm-4
```

---

## Summary

The nocodo manager implements a **production-grade LLM agent system** with:

1. **Provider-agnostic architecture** supporting Claude, OpenAI, xAI, and zAI
2. **Native tool calling** for file operations with error handling
3. **Persistent conversation management** with state reconstruction
4. **Real-time updates** via WebSocket for long-running operations
5. **Structured extensibility** for adding new providers via adapters
6. **Comprehensive logging** for debugging and monitoring

The adapter pattern enables clean separation of concerns, making it straightforward to add new LLM providers without affecting existing code or the application layer.

