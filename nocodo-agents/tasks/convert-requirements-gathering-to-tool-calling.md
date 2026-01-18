# Convert Requirements Gathering Agent from Typed JSON to Tool Calling

**Status**: 📋 Not Started
**Priority**: Medium
**Created**: 2026-01-17

## Summary

Convert the `requirements_gathering` module from using typed JSON responses (`response_format: JsonObject`) to using tool/function calling with an agentic loop pattern. This aligns it with the `sqlite_analysis` agent architecture and provides more flexibility in how the agent interacts with users.

## Problem Statement

### Current Approach (Typed JSON)
The `UserClarificationAgent` currently:
- Forces JSON output via `response_format: JsonObject`
- Includes TypeScript type definitions in the system prompt
- Uses a single-shot interaction with retry loop for JSON validation
- Has no tools defined, no agentic loop
- Parses the final response as `AskUserRequest` struct
- Returns only the JSON output with no natural language explanation

**Limitations:**
- Rigid interaction model (single request/response)
- LLM cannot provide explanations alongside questions
- Cannot have conversational back-and-forth
- No extensibility for adding more tools later

### Desired Approach (Tool Calling)
Following the `sqlite_analysis` pattern:
- Uses `tools` + `tool_choice: Auto` in CompletionRequest
- System prompt describes agent capabilities and objectives
- Agentic loop (up to N iterations) for multi-turn interactions
- LLM decides when to use `ask_user` tool vs. when to respond directly
- Natural language responses with structured tool calls
- Extensible for future tools (e.g., searching docs, checking existing specs)

**Benefits:**
- More flexible and natural interactions
- LLM can explain its reasoning before/after asking questions
- Can decide clarification isn't needed and proceed directly
- Consistent pattern across all agents
- Better user experience

## Goals

1. Convert agent from JSON-forcing to tool calling pattern
2. Implement agentic loop similar to `sqlite_analysis`
3. Add `ToolExecutor` dependency for tool execution
4. Update existing test to work with agentic loop pattern
5. Update `requirements_gathering_runner.rs` binary to instantiate `ToolExecutor`
6. Maintain backward compatibility with `AskUserRequest` when tool is called

## Architecture Overview

### Current Structure
```rust
pub struct UserClarificationAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
}
```

### New Structure (Tool Calling)
```rust
pub struct UserClarificationAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,  // NEW
}
```

### Key Changes

| Component | Current (JSON) | New (Tool Calling) |
|-----------|----------------|-------------------|
| **Response format** | `JsonObject` | `None` (natural) |
| **System prompt** | TypeScript definitions + JSON instructions | Task-oriented objectives |
| **Tools** | Empty vec | `vec![AgentTool::AskUser]` |
| **Execute loop** | Single call + retry on parse error | Agentic loop with tool execution |
| **Output** | JSON only | Natural language + tool calls |
| **Iterations** | Max 3 retries | Max 10 iterations |

## Implementation Plan

### Phase 1: Update Agent Structure

#### 1.1 Add ToolExecutor Dependency
**File**: `nocodo-agents/src/requirements_gathering/mod.rs`

**Update struct (line 18-21):**
```rust
pub struct UserClarificationAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,  // ADD THIS
}
```

**Update constructor (line 24-26):**
```rust
pub fn new(
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,  // ADD THIS
) -> Self {
    Self { client, database, tool_executor }  // UPDATE THIS
}
```

**Update factory function (line 205-211):**
```rust
pub fn create_user_clarification_agent(
    client: Arc<dyn LlmClient>,
) -> anyhow::Result<(UserClarificationAgent, Arc<Database>)> {
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:"))?);
    let tool_executor = Arc::new(manager_tools::ToolExecutor::new(
        manager_tools::BashExecutorImpl,
        None, // No path validator for this agent
    ));
    let agent = UserClarificationAgent::new(client, database.clone(), tool_executor);
    Ok((agent, database))
}
```

**Add imports at top:**
```rust
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::tools::{ToolCall, ToolChoice};
use std::time::Instant;
```

---

### Phase 2: Replace System Prompt

#### 2.1 Update generate_system_prompt()
**File**: `nocodo-agents/src/requirements_gathering/mod.rs`

**Replace method (lines 28-53):**
```rust
fn generate_system_prompt() -> String {
    r#"You are a requirements gathering specialist for business process automation.
Your role is to analyze user requests and determine if clarification is needed before implementation.

CONTEXT:
You are part of a system that helps users define their business processes and automate workflows.
Users will share access to their data sources (databases, APIs, etc.) as needed.

YOUR CAPABILITIES:
- You can ask clarifying questions using the ask_user tool
- You should focus on high-level process understanding, not technical implementation details
- You can ask about data source types/names (not authentication details)
- You can request specific examples (e.g., sample emails, messages to process)
- You should understand the goal and desired outcome of the automation

WHEN TO ASK QUESTIONS:
- The user's goal is unclear or ambiguous
- Critical information about data sources is missing
- The scope of the automation needs definition
- Specific examples would help clarify requirements

WHEN NOT TO ASK QUESTIONS:
- The user has provided a clear, actionable request
- The request is not about business process automation
- You have sufficient information to proceed

If the user's request is clear and describes an automatable software process, respond directly
without using the ask_user tool. Explain that you understand the requirements.

If the user did not share a process that can be automated with software, respond politely
that you need more information about what they want to automate."#.to_string()
}
```

---

### Phase 3: Implement Tool Execution

#### 3.1 Add execute_tool_call Method
**File**: `nocodo-agents/src/requirements_gathering/mod.rs`

**Add method after generate_system_prompt():**
```rust
async fn execute_tool_call(
    &self,
    session_id: i64,
    message_id: Option<i64>,
    tool_call: &ToolCall,
) -> anyhow::Result<()> {
    let tool_request = AgentTool::parse_tool_call(
        tool_call.name(),
        tool_call.arguments().clone(),
    )?;

    let call_id = self.database.create_tool_call(
        session_id,
        message_id,
        tool_call.id(),
        tool_call.name(),
        tool_call.arguments().clone(),
    )?;

    let start = Instant::now();
    let result: anyhow::Result<manager_tools::types::ToolResponse> =
        self.tool_executor.execute(tool_request).await;
    let execution_time = start.elapsed().as_millis() as i64;

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
```

#### 3.2 Add get_tool_definitions Helper
**Add after execute_tool_call:**
```rust
fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
    self.tools()
        .into_iter()
        .map(|tool| tool.to_tool_definition())
        .collect()
}
```

---

### Phase 4: Rewrite Execute Method

#### 4.1 Replace validate_and_retry with Agentic Loop
**File**: `nocodo-agents/src/requirements_gathering/mod.rs`

**Delete methods (lines 55-162):**
- `validate_and_retry()`
- `parse_response()`

**Replace execute() method (lines 179-189) with:**
```rust
async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
    self.database
        .create_message(session_id, "user", user_prompt)?;

    let tools = self.get_tool_definitions();

    let mut iteration = 0;
    let max_iterations = 10;

    loop {
        iteration += 1;
        if iteration > max_iterations {
            let error = "Maximum iteration limit reached";
            self.database.fail_session(session_id, error)?;
            return Err(anyhow::anyhow!(error));
        }

        let messages = self.build_messages(session_id)?;

        let request = CompletionRequest {
            messages,
            max_tokens: 2000,
            model: self.client.model_name().to_string(),
            system: Some(self.system_prompt()),
            temperature: Some(0.3),
            top_p: None,
            stop_sequences: None,
            tools: Some(tools.clone()),
            tool_choice: Some(ToolChoice::Auto),
            response_format: None,  // Remove JSON forcing
        };

        let response = self.client.complete(request).await?;

        let text = extract_text_from_content(&response.content);

        // If there's no text but there are tool calls, use a placeholder for storage
        let text_to_save = if text.is_empty() && response.tool_calls.is_some() {
            "[Using tools]"
        } else {
            &text
        };

        let message_id = self
            .database
            .create_message(session_id, "assistant", text_to_save)?;

        if let Some(tool_calls) = response.tool_calls {
            if tool_calls.is_empty() {
                self.database.complete_session(session_id, &text)?;
                return Ok(text);
            }

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
```

#### 4.2 Update build_messages to Read from Database
**Replace build_messages method (lines 120-148) with:**
```rust
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
```

---

### Phase 5: Update Agent Trait Implementation

#### 5.1 Update tools() Method
**File**: `nocodo-agents/src/requirements_gathering/mod.rs`

**Replace method (lines 175-177):**
```rust
fn tools(&self) -> Vec<AgentTool> {
    vec![AgentTool::AskUser]  // Add the ask_user tool
}
```

---

### Phase 6: Update Tests

#### 6.1 Add MockToolExecutor
**File**: `nocodo-agents/src/requirements_gathering/tests.rs`

**Add after imports (line 9):**
```rust
use manager_tools::types::{ToolRequest, ToolResponse};
use manager_tools::{BashExecutorTrait, ToolExecutor};

struct MockBashExecutor;

#[async_trait::async_trait]
impl BashExecutorTrait for MockBashExecutor {
    async fn execute(
        &self,
        _command: &str,
    ) -> Result<manager_tools::types::BashResponse, manager_tools::error::ToolError> {
        unreachable!("Bash execution should not be called in these tests")
    }
}

struct MockToolExecutor;

#[async_trait::async_trait]
impl manager_tools::ToolExecutorTrait for MockToolExecutor {
    async fn execute(
        &self,
        request: ToolRequest,
    ) -> anyhow::Result<ToolResponse> {
        match request {
            ToolRequest::AskUser(ask_req) => {
                // Simulate user providing answers
                let mut responses = std::collections::HashMap::new();
                for question in &ask_req.questions {
                    responses.insert(question.id.clone(), "Mock answer".to_string());
                }
                Ok(ToolResponse::AskUser(shared_types::user_interaction::AskUserResponse {
                    responses,
                }))
            }
            _ => Err(anyhow::anyhow!("Unexpected tool call")),
        }
    }
}
```

#### 6.2 Update MockLlmClient to Support Tool Calls
**Replace MockLlmClient (lines 10-38) with:**
```rust
struct MockLlmClient {
    response_content: String,
    include_tool_call: bool,
}

#[async_trait::async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let tool_calls = if self.include_tool_call {
            Some(vec![nocodo_llm_sdk::tools::ToolCall::new(
                "call_123".to_string(),
                "ask_user".to_string(),
                serde_json::json!({
                    "questions": [
                        {
                            "id": "q1",
                            "question": "What is the primary purpose of the website?",
                            "type": "text",
                            "description": "e.g., portfolio, e-commerce, blog"
                        }
                    ]
                }),
            )])
        } else {
            None
        };

        Ok(CompletionResponse {
            content: vec![ContentBlock::Text {
                text: self.response_content.clone(),
            }],
            role: Role::Assistant,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 20,
            },
            stop_reason: Some("end_turn".to_string()),
            tool_calls,
        })
    }

    fn provider_name(&self) -> &str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }
}
```

#### 6.3 Update setup_test_agent
**Replace setup_test_agent (lines 40-51) with:**
```rust
fn setup_test_agent(
    response_content: &str,
    include_tool_call: bool,
) -> anyhow::Result<(UserClarificationAgent, Arc<Database>)> {
    let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient {
        response_content: response_content.to_string(),
        include_tool_call,
    });

    let database = Arc::new(Database::new(&PathBuf::from(":memory:"))?);
    let tool_executor = Arc::new(ToolExecutor::new(MockBashExecutor, None));
    let agent = UserClarificationAgent::new(client, database.clone(), tool_executor);

    Ok((agent, database))
}
```

#### 6.4 Update Existing Test
**Replace test (lines 53-103) with:**
```rust
#[tokio::test]
async fn test_user_clarification_agent_uses_ask_user_tool() {
    // Mock LLM will call ask_user tool
    let mock_response = "I need to gather some information about your requirements.";

    let (agent, database) = setup_test_agent(mock_response, true).unwrap();

    let session_id = database
        .create_session(
            "user-clarification",
            "test",
            "test",
            None,
            "Build me a website",
            None,
        )
        .unwrap();

    let result = agent
        .execute("Build me a website", session_id)
        .await
        .unwrap();

    // The result should contain the natural language response
    assert!(result.contains("requirements") || result.contains("information"));

    // Verify tool call was made
    let messages = database.get_messages(session_id).unwrap();
    let tool_messages: Vec<_> = messages
        .iter()
        .filter(|m| m.role == "tool")
        .collect();

    assert!(!tool_messages.is_empty(), "Expected at least one tool call message");
}
```


---

### Phase 7: Update Runner Binary

#### 7.1 Update requirements_gathering_runner.rs
**File**: `nocodo-agents/bin/requirements_gathering_runner.rs`

**Add imports (after line 7):**
```rust
use manager_tools::{BashExecutorImpl, ToolExecutor};
```

**Replace agent creation (lines 42-44) with:**
```rust
let tool_executor = Arc::new(ToolExecutor::new(
    BashExecutorImpl,
    None, // No path validator for this agent
));

// Note: create_user_clarification_agent now creates its own tool_executor
// If you want to pass a custom one, you'll need to update the factory function
let (agent, database) = create_user_clarification_agent(client)?;
```

**Alternative: If you want custom tool executor in runner, update create function signature:**
```rust
// In mod.rs, update factory function to accept optional ToolExecutor:
pub fn create_user_clarification_agent(
    client: Arc<dyn LlmClient>,
    tool_executor: Option<Arc<ToolExecutor>>,
) -> anyhow::Result<(UserClarificationAgent, Arc<Database>)> {
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:"))?);
    let tool_executor = tool_executor.unwrap_or_else(|| {
        Arc::new(manager_tools::ToolExecutor::new(
            manager_tools::BashExecutorImpl,
            None,
        ))
    });
    let agent = UserClarificationAgent::new(client, database.clone(), tool_executor);
    Ok((agent, database))
}

// Then in runner:
let tool_executor = Arc::new(ToolExecutor::new(BashExecutorImpl, None));
let (agent, database) = create_user_clarification_agent(client, Some(tool_executor))?;
```

---

## Testing Plan

### Unit Tests
1. **Update existing test**: Modify `test_user_clarification_agent_returns_questions_when_needed` to work with tool calling pattern
2. **Verify tool calling**: Ensure agent calls `ask_user` tool and processes results correctly
3. **Verify database logging**: Ensure tool calls are properly recorded in database

### Integration Tests
```bash
# Test with runner binary
cargo run --bin requirements_gathering_runner -- \
  --prompt "Build me a website" \
  --config /path/to/config.toml

# Expected: Agent completes successfully with tool calling mechanism
```

### Manual Verification
- [ ] Agent calls `ask_user` tool when appropriate
- [ ] Tool execution results are properly processed and logged
- [ ] Agentic loop functions correctly
- [ ] Database captures all tool calls and responses

---

## Rollout Strategy

### Phase 1: Implementation (This Task)
1. Convert agent to tool calling pattern
2. Update tests
3. Update runner binary
4. Verify all tests pass

### Phase 2: Validation
1. Compare behavior with previous JSON-based version
2. Test with various user prompts
3. Verify database logging captures tool calls correctly
4. Ensure agentic loop handles tool execution properly

### Phase 3: Documentation
1. Update agent documentation to reflect tool calling approach
2. Document the agentic loop implementation
3. Add examples of tool calling interactions

---

## Success Criteria

- [ ] Agent compiles without errors
- [ ] Existing test passes (updated to new pattern)
- [ ] Runner binary works with new agent structure
- [ ] Agent calls `ask_user` tool appropriately
- [ ] Database properly logs tool calls and responses
- [ ] Agentic loop functions correctly with max iterations

---

## References

- **sqlite_analysis agent**: Reference implementation for tool calling pattern (nocodo-agents/src/sqlite_analysis/mod.rs)
- **AgentTool enum**: Tool definitions and parsing (nocodo-agents/src/lib.rs:18-109)
- **Current implementation**: JSON-based agent (nocodo-agents/src/requirements_gathering/mod.rs)
