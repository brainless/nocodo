# LLM Client Refactor Process with E2E Testing

## Executive Summary

This document outlines the step-by-step process for refactoring each LLM provider/model using the Adapter Pattern (as detailed in `LLM_CLIENT_REFACTOR.md`) with e2e test validation at each step. The existing e2e test will serve as our **acceptance criteria** - each model must pass the test after its refactor is complete.

**Key Principle**: We refactor one model at a time, test it, fix any issues, and only then move to the next model. This ensures we don't break existing functionality and can isolate any issues that arise.

---

## Test Overview

### E2E Test File
- **Location**: `manager/tests/llm_e2e_real_test.rs`
- **Runner Script**: `./run_llm_e2e_test.sh <provider> <model>`
- **Test Name**: `test_llm_e2e_saleor`

### What the Test Does

1. **Sets up isolated test environment** with test database and project
2. **Creates a project from Saleor git repository** (clones `git@github.com:saleor/saleor.git`)
3. **Creates a work session** with the specified model
4. **Sends a prompt** asking the LLM to analyze the tech stack
5. **LLM should use tools** to read files like:
   - `manage.py`
   - `pyproject.toml`
   - `requirements.txt`
   - `setup.py`
   - Other configuration files
6. **Validates the response** using keyword matching
7. **Checks for required keywords** in the response

### Test Acceptance Criteria

#### Required Keywords (ALL must be present)
- `Django`
- `Python`
- `PostgreSQL`
- `GraphQL`

#### Optional Keywords (SOME should be present)
- `JavaScript`
- `Node`

#### Forbidden Keywords (NONE should be present)
- *(currently empty)*

#### Minimum Score
- **0.7** out of 1.0
- Score calculation:
  - Required keywords: 70% weight
  - Optional keywords: 20% weight
  - Forbidden penalty: -10% per keyword

#### Additional Validation
- Response length > 50 characters
- Response is not just an error message
- LLM must make tool calls to read files
- Final response must contain technical content

### Expected Test Flow

```
1. Test creates project from git repo
2. Test sends prompt: "What is the tech stack of this project?
   You must examine at least 3 different configuration files..."
3. LLM uses tools:
   - list_files (to see directory structure)
   - read_file (to read manage.py)
   - read_file (to read pyproject.toml)
   - read_file (to read requirements.txt)
   - read_file (to read other config files)
4. LLM synthesizes information from files
5. LLM provides response containing:
   "The project uses Django (Python web framework),
    PostgreSQL for database, and GraphQL for API..."
6. Test validates response contains all 4 required keywords
7. Test passes ✅
```

---

## Current State Analysis

### Working Models
- **grok-code-fast-1** (xAI) - Currently working ✅
- **gpt-4** (OpenAI) - Should work ✅

### Broken Models
- **gpt-5-codex** (OpenAI) - Recently fixed but needs adapter ⚠️
- **claude-3-sonnet** (Anthropic) - Broken due to gpt-5-codex changes ❌
- **claude-3-haiku** (Anthropic) - Broken due to gpt-5-codex changes ❌

### Models in E2E Test Config
From `manager/tests/common/llm_config.rs`:
- **xAI**: `grok-code-fast-1`
- **OpenAI**: `gpt-5`, `gpt-5-codex`
- **Anthropic**: `claude-sonnet-4-20250514`, `claude-3-sonnet-20240229`, `claude-3-haiku-20240307`

---

## Refactoring Order

We'll refactor in this order to minimize risk:

### Phase 1: Foundation (1 day)
1. ✅ Create adapter infrastructure (no model changes)
2. ✅ Run e2e test with existing code to establish baseline

### Phase 2: Migrate Working Models (2 days)
3. **ChatCompletionsAdapter** (OpenAI standard models)
   - Test with: `grok-code-fast-1`
   - Test with: `gpt-4` (if available)
4. **GrokAdapter** (Optional - can reuse ChatCompletions)
   - Test with: `grok-code-fast-1`

### Phase 3: Migrate Complex Models (2 days)
5. **ResponsesApiAdapter** (GPT-5-codex)
   - Test with: `gpt-5-codex`
6. **ClaudeMessagesAdapter** (Anthropic/Claude)
   - Test with: `claude-3-sonnet-20240229`
   - Test with: `claude-3-haiku-20240307`
   - Test with: `claude-sonnet-4-20250514`

### Phase 4: Final Integration (1 day)
7. Run full test suite for all models
8. Document any model-specific quirks
9. Update documentation

---

## Step-by-Step Process for Each Model

### Template for Each Model Refactor

```
┌─────────────────────────────────────────────────────────────────┐
│                     Model: <model-name>                         │
│                   Adapter: <adapter-name>                       │
│                  Provider: <provider-name>                      │
└─────────────────────────────────────────────────────────────────┘

STEP 1: Pre-Refactor Test (Establish Baseline)
───────────────────────────────────────────────────────────────────
□ Ensure API key is configured
□ Run test with current code:
  ./run_llm_e2e_test.sh <provider> <model>
□ Document current state:
  - [ ] PASS - Already working
  - [ ] FAIL - Known broken
  - [ ] SKIP - No API key
□ If FAIL, note the error for comparison later

STEP 2: Implement Adapter
───────────────────────────────────────────────────────────────────
□ Create adapter file: manager/src/llm_client/adapters/<name>.rs
□ Implement ProviderAdapter trait:
  - get_api_url()
  - supports_native_tools()
  - prepare_request()
  - send_request()
  - parse_response()
  - extract_tool_calls()
  - provider_name()
  - model_name()
□ Move model-specific logic from old client
□ Add unit tests for adapter

STEP 3: Update Factory
───────────────────────────────────────────────────────────────────
□ Update create_llm_client() to use new adapter for this model
□ Keep old client as fallback initially
□ Compile and fix any errors

STEP 4: Post-Refactor Test (Validate)
───────────────────────────────────────────────────────────────────
□ Run test with new adapter:
  ./run_llm_e2e_test.sh <provider> <model>
□ Check test output:
  - [ ] PASS - Refactor successful! ✅
  - [ ] FAIL - Debug required ❌
□ If FAIL, review:
  1. Request format - Is it correct for this provider?
  2. Response parsing - Are we extracting fields correctly?
  3. Tool call format - Are tool calls structured properly?
  4. Message conversion - Are we handling all message types?

STEP 5: Debug (If Test Fails)
───────────────────────────────────────────────────────────────────
□ Add debug logging to adapter
□ Compare raw API request/response with working model
□ Check tool call extraction logic
□ Verify message conversion for conversation history
□ Run test again after each fix

STEP 6: Validation Complete
───────────────────────────────────────────────────────────────────
□ Test passes consistently (3+ runs)
□ Unit tests pass
□ Integration tests pass
□ Document any model-specific quirks
□ Commit changes for this model
□ Move to next model
```

---

## Detailed Refactoring Steps by Model

### Model 1: grok-code-fast-1 (xAI/Grok)

**Status**: Currently working ✅
**Adapter**: `ChatCompletionsAdapter` (or `GrokAdapter`)
**Priority**: High (establish adapter pattern with working model)

#### Pre-Refactor Test
```bash
# Set up API key
export GROK_API_KEY="your-api-key"

# Or ensure it's in ~/.config/nocodo/manager.toml:
[api_keys]
xai_api_key = "your-api-key"

# Run test with current implementation
./run_llm_e2e_test.sh xai grok-code-fast-1
```

**Expected Result**: ✅ PASS (baseline - currently working)

#### Implementation Steps

1. **Create ChatCompletionsAdapter**
   ```bash
   # File: manager/src/llm_client/adapters/chat_completions.rs

   # Extract logic from OpenAiCompatibleClient:
   - Standard OpenAI Chat Completions API
   - Native tool calling support
   - Legacy function calling support
   - Request/response conversion
   ```

2. **Update Factory**
   ```rust
   // In manager/src/llm_client/mod.rs
   pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
       match (config.provider.to_lowercase().as_str(), config.model.as_str()) {
           ("xai" | "grok", _) => {
               let adapter = Box::new(adapters::ChatCompletionsAdapter::new(config.clone())?);
               Ok(Box::new(UnifiedLlmClient::new(adapter, config)?))
           }
           // ... other providers remain unchanged for now
           _ => {
               // Fall back to old implementation temporarily
               create_llm_client_legacy(config)
           }
       }
   }
   ```

3. **Post-Refactor Test**
   ```bash
   # Run test with new adapter
   ./run_llm_e2e_test.sh xai grok-code-fast-1
   ```

**Expected Result**: ✅ PASS

**Acceptance Criteria**:
- Test creates project from Saleor git repo
- LLM makes tool calls (list_files, read_file)
- Response contains: Django, Python, PostgreSQL, GraphQL
- Validation score >= 0.7
- No regressions in tool calling

---

### Model 2: gpt-5-codex (OpenAI Responses API)

**Status**: Recently fixed but needs adapter ⚠️
**Adapter**: `ResponsesApiAdapter`
**Priority**: High (complex model, recent changes)

#### Pre-Refactor Test
```bash
# Set up API key
export OPENAI_API_KEY="your-api-key"

# Run test with current implementation
./run_llm_e2e_test.sh openai gpt-5-codex
```

**Expected Result**: Should work with current code (if it doesn't, this is high priority to fix)

#### Implementation Steps

1. **Create ResponsesApiAdapter**
   ```bash
   # File: manager/src/llm_client/adapters/responses_api.rs

   # Extract logic specific to gpt-5-codex:
   - Uses v1/responses endpoint (not v1/chat/completions)
   - Different request format (instructions, input array)
   - Different response format (output items, function_call items)
   - Custom Codex instructions
   - Tool definition with strict mode
   ```

2. **Key Differences to Handle**
   ```rust
   // Request format
   ResponsesApiRequest {
       model: "gpt-5-codex",
       instructions: "Codex-specific instructions...",  // System prompt goes here
       input: [/* messages as JSON */],                   // Not "messages"
       tools: [/* ResponsesToolDefinition */],           // Different format
       tool_choice: "auto",                              // String, not enum
       stream: false,
   }

   // Response format
   ResponsesApiResponse {
       output: [
           ResponseItem::Message { content: [...] },
           ResponseItem::FunctionCall { name, arguments, call_id },
           ResponseItem::Reasoning { summary },
       ]
   }
   ```

3. **Update Factory**
   ```rust
   match (config.provider.to_lowercase().as_str(), config.model.as_str()) {
       ("openai", "gpt-5-codex") => {
           let adapter = Box::new(adapters::ResponsesApiAdapter::new(config.clone())?);
           Ok(Box::new(UnifiedLlmClient::new(adapter, config)?))
       }
       // ...
   }
   ```

4. **Post-Refactor Test**
   ```bash
   ./run_llm_e2e_test.sh openai gpt-5-codex
   ```

**Expected Result**: ✅ PASS

**Acceptance Criteria**:
- Correctly uses `/v1/responses` endpoint
- Sends `instructions` field with Codex-specific prompt
- Sends `input` array (not `messages`)
- Properly converts function calls from output array
- Tool calls work correctly
- Response contains: Django, Python, PostgreSQL, GraphQL

**Known Gotchas**:
1. **Instructions field**: Must include Codex-specific instructions about shell commands, file operations, etc.
2. **Tool format**: Tools have `strict: true` and use ResponsesToolDefinition
3. **Message conversion**: System messages go in `instructions`, not in `input`
4. **Tool results**: Must use `tool_call_id` correctly in conversation history
5. **Response parsing**: Must aggregate multiple output items into single response

---

### Model 3: claude-3-sonnet-20240229 (Anthropic)

**Status**: Broken due to gpt-5-codex changes ❌
**Adapter**: `ClaudeMessagesAdapter`
**Priority**: High (currently broken, needs immediate fix)

#### Pre-Refactor Test
```bash
# Set up API key
export ANTHROPIC_API_KEY="your-api-key"

# Run test with current implementation
./run_llm_e2e_test.sh anthropic claude-3-sonnet-20240229
```

**Expected Result**: ❌ FAIL (currently broken)

**Likely Error**: Tool calling not working correctly due to gpt-5-codex changes affecting shared code paths

#### Implementation Steps

1. **Create ClaudeMessagesAdapter**
   ```bash
   # File: manager/src/llm_client/adapters/claude_messages.rs

   # Extract Claude-specific logic:
   - Uses v1/messages endpoint
   - Content blocks instead of simple text
   - Different tool definition format (input_schema)
   - Tool calls as ContentBlock::ToolUse
   - Tool results as ContentBlock::ToolResult
   - System prompt in separate field
   ```

2. **Key Differences to Handle**
   ```rust
   // Request format
   ClaudeCompletionRequest {
       model: "claude-3-sonnet-20240229",
       system: "System prompt here",  // Separate field, not in messages
       messages: [
           ClaudeMessage {
               role: "user",
               content: [
                   ClaudeContentBlock::Text { text: "..." }
               ]
           },
           ClaudeMessage {
               role: "assistant",
               content: [
                   ClaudeContentBlock::Text { text: "..." },
                   ClaudeContentBlock::ToolUse {
                       id: "...",
                       name: "read_file",
                       input: { "path": "..." }
                   }
               ]
           },
           ClaudeMessage {
               role: "user",
               content: [
                   ClaudeContentBlock::ToolResult {
                       tool_use_id: "...",
                       content: "..."
                   }
               ]
           }
       ],
       tools: [
           ClaudeToolDefinition {
               name: "read_file",
               description: "...",
               input_schema: { /* JSON schema */ }
           }
       ]
   }

   // Response format
   ClaudeCompletionResponse {
       content: [
           ClaudeContentBlock::Text { text: "..." },
           ClaudeContentBlock::ToolUse { id, name, input }
       ],
       stop_reason: "tool_use" or "end_turn"
   }
   ```

3. **Message Conversion Logic**
   ```rust
   // Convert LlmMessage to ClaudeMessage
   fn convert_to_claude_message(&self, message: &LlmMessage) -> ClaudeMessage {
       match message.role.as_str() {
           "assistant" => {
               // Handle stored tool call data
               if let Some(content_str) = &message.content {
                   if let Ok(assistant_data) = serde_json::from_str::<Value>(content_str) {
                       if let (Some(text), Some(tool_calls)) = (...) {
                           // Build content blocks with text + tool_use blocks
                       }
                   }
               }
           }
           "tool" => {
               // Convert tool results to ToolResult content block
               if let Some(content_str) = &message.content {
                   if let Ok(tool_result_data) = serde_json::from_str::<Value>(content_str) {
                       // Extract tool_use_id and content
                       // Create ToolResult content block
                   }
               }
           }
           // ...
       }
   }
   ```

4. **Update Factory**
   ```rust
   match (config.provider.to_lowercase().as_str(), config.model.as_str()) {
       ("anthropic" | "claude", _) => {
           let adapter = Box::new(adapters::ClaudeMessagesAdapter::new(config.clone())?);
           Ok(Box::new(UnifiedLlmClient::new(adapter, config)?))
       }
       // ...
   }
   ```

5. **Post-Refactor Test**
   ```bash
   ./run_llm_e2e_test.sh anthropic claude-3-sonnet-20240229
   ```

**Expected Result**: ✅ PASS (should be fixed)

**Acceptance Criteria**:
- Correctly uses `/v1/messages` endpoint
- System prompt in `system` field (not in messages array)
- Messages use content blocks (not simple strings)
- Tool calls converted to ToolUse content blocks
- Tool results converted to ToolResult content blocks
- Conversation history properly reconstructed
- Response contains: Django, Python, PostgreSQL, GraphQL

**Known Gotchas**:
1. **Content blocks**: Everything must be wrapped in content blocks
2. **System messages**: Go in separate `system` field, NOT in messages array
3. **Tool results**: Must be sent as user role with ToolResult content block
4. **Tool use ID**: Must match between ToolUse and ToolResult
5. **Message alternation**: Claude requires strict user/assistant alternation
6. **Conversation reconstruction**: Must properly parse stored tool call data from database

---

### Model 4: claude-3-haiku-20240307 (Anthropic)

**Status**: Broken due to gpt-5-codex changes ❌
**Adapter**: `ClaudeMessagesAdapter` (same as sonnet)
**Priority**: Medium (same adapter as sonnet)

#### Implementation

Once `ClaudeMessagesAdapter` is working for `claude-3-sonnet-20240229`, this model should automatically work since they use the same adapter.

#### Test
```bash
./run_llm_e2e_test.sh anthropic claude-3-haiku-20240307
```

**Expected Result**: ✅ PASS (if sonnet passes)

---

### Model 5: claude-sonnet-4-20250514 (Anthropic)

**Status**: Unknown (newest model)
**Adapter**: `ClaudeMessagesAdapter` (same as other Claude models)
**Priority**: Low (newest model, should work if adapter is correct)

#### Test
```bash
./run_llm_e2e_test.sh anthropic claude-sonnet-4-20250514
```

**Expected Result**: ✅ PASS (if other Claude models pass)

---

### Model 6: gpt-4 (OpenAI) [Optional]

**Status**: Should work ✅
**Adapter**: `ChatCompletionsAdapter`
**Priority**: Low (if available, good to test)

#### Test
```bash
./run_llm_e2e_test.sh openai gpt-4
```

**Expected Result**: ✅ PASS

---

## Debugging Guide

### Common Test Failures

#### 1. Tool Calls Not Working

**Symptom**: LLM doesn't make any tool calls, or tool calls fail

**Debug Steps**:
```bash
# Enable debug logging
export RUST_LOG=debug

# Run test
./run_llm_e2e_test.sh <provider> <model>

# Look for:
# - "Sending request via adapter" - Request being sent
# - "Raw LLM request being sent" - Actual API request
# - "Raw LLM response received" - API response
# - "Found tool calls in message" - Tool call extraction
# - "Processing native tool call" - Tool execution
```

**Common Causes**:
1. Tools not included in request
2. Tool format incorrect for provider
3. Tool choice set to "none"
4. Tool call extraction not finding tool calls in response
5. Provider-specific tool format differences

**Fixes**:
```rust
// Check: Are tools being sent?
tracing::debug!("Request tools: {:?}", request.tools);

// Check: Tool call extraction
let tool_calls = adapter.extract_tool_calls(&response);
tracing::debug!("Extracted {} tool calls", tool_calls.len());

// Check: Response structure
tracing::debug!("Response choices: {:?}", response.choices);
for choice in &response.choices {
    tracing::debug!("Choice message: {:?}", choice.message);
    tracing::debug!("Choice tool_calls: {:?}", choice.tool_calls);
}
```

#### 2. Keywords Missing from Response

**Symptom**: Test fails with "Missing required keywords: [Django, Python, ...]"

**Debug Steps**:
```bash
# Check AI outputs in test
# The test prints each output as it arrives

# Look for:
# - Tool call outputs (list_files, read_file)
# - File contents in outputs
# - Final text response

# Check if LLM is:
# - Reading the right files (manage.py, pyproject.toml, etc.)
# - Extracting tech stack information
# - Providing a final summary
```

**Common Causes**:
1. LLM not reading enough files
2. LLM not synthesizing information
3. LLM providing tool outputs but no final text response
4. Files not containing expected keywords

**Fixes**:
- Improve system prompt to require comprehensive analysis
- Ensure LLM reads multiple configuration files
- Check that tool results are properly sent back to LLM
- Verify conversation history includes all tool calls and results

#### 3. Request Format Incorrect

**Symptom**: API returns 400/422 error

**Debug Steps**:
```bash
# Check raw API request in logs
# Look for "Raw LLM request being sent"

# Compare with provider's API documentation
# - OpenAI Chat Completions: https://platform.openai.com/docs/api-reference/chat
# - OpenAI Responses: https://platform.openai.com/docs/api-reference/responses
# - Anthropic Messages: https://docs.anthropic.com/claude/reference/messages_post
```

**Common Causes**:
1. Field names incorrect
2. Field types incorrect (string vs enum)
3. Required fields missing
4. Extra fields not supported by provider

**Fixes**:
```rust
// Validate request structure matches provider's API spec
#[derive(Debug, Serialize)]
struct ProviderSpecificRequest {
    // Fields must match provider's API exactly
}

// Add validation
impl ProviderAdapter for MyAdapter {
    fn prepare_request(&self, request: LlmCompletionRequest)
        -> Result<Box<dyn ProviderRequest>> {
        // Validate before sending
        let provider_request = self.convert_request(request)?;

        // Log for debugging
        tracing::debug!(
            "Provider request: {}",
            serde_json::to_string_pretty(&provider_request)?
        );

        Ok(Box::new(provider_request))
    }
}
```

#### 4. Response Parsing Fails

**Symptom**: Error parsing API response

**Debug Steps**:
```bash
# Check raw API response in logs
# Look for "Raw LLM response received"

# Check error message
# - JSON parse error?
# - Field missing?
# - Field type mismatch?
```

**Common Causes**:
1. Response structure changed by provider
2. Optional fields assumed to be required
3. Field type mismatches (string vs number)
4. Nested structures not handled

**Fixes**:
```rust
// Use serde defaults for optional fields
#[derive(Debug, Deserialize)]
struct ProviderResponse {
    pub id: String,

    #[serde(default)]  // Use default if missing
    pub model: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

// Add custom deserialization for complex types
impl<'de> Deserialize<'de> for ComplexField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        // Custom parsing logic
    }
}
```

#### 5. Tool Results Not Sent Back

**Symptom**: LLM makes tool call, tool executes, but LLM doesn't receive result

**Debug Steps**:
```bash
# Check llm_agent logs
# Look for:
# - "Processing native tool call" - Tool execution
# - "Tool result: ..." - Tool output
# - "Sending message to LLM" - Next request with tool result
```

**Common Causes**:
1. Tool result message not created
2. Tool result format incorrect for provider
3. tool_call_id mismatch
4. Conversation history not including tool results

**Fixes**:
```rust
// Ensure tool results are stored correctly
let tool_result_message = serde_json::json!({
    "tool_use_id": tool_call.id,  // or "tool_call_id" for OpenAI
    "content": tool_output
});

// Store in database
db.create_llm_agent_message(
    session_id,
    "tool",  // Role must be "tool"
    serde_json::to_string(&tool_result_message)?
)?;
```

---

## Test Checklist for Each Model

Use this checklist when testing each model:

```
Model: ___________________________
Provider: _________________________
Adapter: __________________________

PRE-REFACTOR
□ API key configured
□ Test runs with current code
□ Current state documented (PASS/FAIL)
□ If FAIL, error message documented

IMPLEMENTATION
□ Adapter file created
□ ProviderAdapter trait implemented
□ Model-specific logic moved
□ Unit tests written
□ Factory updated
□ Code compiles without errors

POST-REFACTOR
□ Test runs with new adapter
□ Test passes 3+ times consistently

TEST VALIDATION
□ Project created from git repo
□ Work session created successfully
□ AI session created successfully
□ Tool calls made:
  □ list_files called
  □ read_file called (3+ times)
  □ Files read: manage.py, pyproject.toml, etc.
□ Response received
□ Keywords found:
  □ Django
  □ Python
  □ PostgreSQL
  □ GraphQL
□ Validation score >= 0.7
□ Response length > 50 chars
□ No error messages in response

CLEANUP
□ Debug logging removed
□ Unit tests pass
□ Integration tests pass
□ Documentation updated
□ Changes committed

SIGN-OFF
□ Model works correctly
□ Ready for next model
```

---

## Running Tests

### Individual Model Test

```bash
# Set API key (one of these methods)
export GROK_API_KEY="your-key"
export OPENAI_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"

# Or add to ~/.config/nocodo/manager.toml:
[api_keys]
xai_api_key = "your-key"
openai_api_key = "your-key"
anthropic_api_key = "your-key"

# Run test for specific model
./run_llm_e2e_test.sh <provider> <model>

# Examples:
./run_llm_e2e_test.sh xai grok-code-fast-1
./run_llm_e2e_test.sh openai gpt-5-codex
./run_llm_e2e_test.sh anthropic claude-3-sonnet-20240229
```

### All Models Test

```bash
# Test all available models
for provider_model in "xai grok-code-fast-1" \
                      "openai gpt-5-codex" \
                      "anthropic claude-3-sonnet-20240229"; do
    read provider model <<< "$provider_model"
    echo "Testing $provider - $model"
    ./run_llm_e2e_test.sh $provider $model
    echo "---"
done
```

### With Debug Logging

```bash
# Enable debug logging
export RUST_LOG=debug

# Run test
./run_llm_e2e_test.sh <provider> <model>

# Save output to file
./run_llm_e2e_test.sh <provider> <model> 2>&1 | tee test_output.log
```

---

## Success Metrics

### Per-Model Metrics

For each model, track:
- ✅ **Pre-refactor state**: PASS/FAIL/SKIP
- ✅ **Post-refactor state**: PASS/FAIL
- ✅ **Validation score**: 0.0 - 1.0
- ✅ **Keywords found**: 0 - 4 (required)
- ✅ **Tool calls made**: Number of tool calls
- ✅ **Files read**: Number of configuration files read
- ✅ **Response quality**: Length and content

### Overall Metrics

Track across all models:
- **Models tested**: 5-7 models
- **Models passing**: Target 100%
- **Average score**: Target >= 0.8
- **Test reliability**: Should pass 95%+ of the time

---

## Timeline

### Optimistic Timeline (All models pass quickly)
- **Day 1**: Foundation + grok-code-fast-1
- **Day 2**: gpt-5-codex
- **Day 3**: Claude models (all 3)
- **Day 4**: Final testing and documentation

### Realistic Timeline (Some debugging required)
- **Day 1**: Foundation + grok-code-fast-1 (baseline)
- **Day 2**: grok-code-fast-1 adapter + gpt-5-codex start
- **Day 3**: gpt-5-codex adapter (complex API)
- **Day 4**: Claude sonnet adapter
- **Day 5**: Claude haiku + sonnet-4 (reuse adapter)
- **Day 6**: Final testing, debugging, documentation

### Pessimistic Timeline (Multiple issues)
- **Days 1-2**: Foundation + grok (baseline)
- **Days 3-4**: gpt-5-codex (complex, may need debugging)
- **Days 5-7**: Claude models (currently broken, need fixes)
- **Days 8-9**: Integration testing and bug fixes
- **Day 10**: Documentation and cleanup

---

## Risk Mitigation

### Risk: Test Fails After Refactor

**Mitigation**:
1. Keep old client code during migration
2. Add feature flag to switch between old/new
3. Compare requests/responses between old and new
4. Roll back if issues persist

### Risk: Tool Calling Breaks

**Mitigation**:
1. Test tool calling separately before full e2e
2. Add debug logging for tool call extraction
3. Verify with provider's API examples
4. Test conversation history reconstruction

### Risk: Provider API Changes

**Mitigation**:
1. Pin provider library versions
2. Document API version used
3. Add API version checking
4. Monitor provider changelogs

### Risk: Timeout Issues

**Mitigation**:
1. Increase test timeout to 240 seconds (currently in test)
2. Add retry logic for transient failures
3. Test with faster models first
4. Use smaller test repository if needed

---

## Post-Refactor Validation

After all models are refactored:

### 1. Full Test Suite
```bash
# Run all tests
cd manager
cargo test

# Run integration tests
cargo test --test '*'

# Run e2e tests for all models
for provider_model in "xai grok-code-fast-1" \
                      "openai gpt-5-codex" \
                      "anthropic claude-3-sonnet-20240229"; do
    ./run_llm_e2e_test.sh $provider $model
done
```

### 2. Performance Testing
```bash
# Measure request latency
time ./run_llm_e2e_test.sh xai grok-code-fast-1

# Compare before/after refactor
# Should be < 5% overhead
```

### 3. Memory Testing
```bash
# Check for memory leaks
valgrind --leak-check=full cargo test
```

### 4. Load Testing
```bash
# Run multiple tests concurrently
for i in {1..5}; do
    ./run_llm_e2e_test.sh xai grok-code-fast-1 &
done
wait
```

---

## Documentation Updates

After refactor is complete:

### 1. Update README
- Document new adapter pattern
- Add examples for each provider
- Update troubleshooting guide

### 2. Update API Documentation
- Document adapter interface
- Add provider-specific notes
- Update configuration examples

### 3. Update Testing Documentation
- Document e2e test process
- Add debugging guides
- Update CI/CD integration

---

## Appendix A: Test Output Examples

### Successful Test Output

```
🚀 Running LLM E2E test with provider: xai
   Model: grok-code-fast-1

📦 Phase 1: Setting up isolated test environment
   ✅ Test isolation configured with ID: test-abc123
   ✅ LLM agent configured

🤖 Phase 2: Setting up real LLM integration
   ✅ Created work session: 1
   ✅ Added initial message: 1
   ✅ Created AI session: 1
   ✅ Project context created from git repository: git@github.com:saleor/saleor.git

🎯 Phase 3: Testing LLM interaction with keyword validation
   📤 Prompt sent to AI session: What is the tech stack of this project?...
   ⏳ Waiting for AI session response...
   📝 New AI output: {"type":"list_files","files":"..."}
   📝 New AI output: {"type":"read_file","path":"manage.py","content":"..."}
   📝 New AI output: {"type":"read_file","path":"pyproject.toml","content":"..."}
   📝 New AI output: The project uses Django as the web framework, Python as the primary language, PostgreSQL for the database, and GraphQL for the API layer...
   ✅ AI text response received after 8 attempts (40 seconds)
   📥 LLM Response received (523 chars)
   📝 Response preview: The project uses Django as the web framework, Python as the primary language...

🔍 Phase 3: Validating LLM response with keyword matching
   📊 Validation Results:
      • Score: 0.85
      • Required keywords found: ["Django", "Python", "PostgreSQL", "GraphQL"]
      • Optional keywords found: ["JavaScript"]
      • Forbidden keywords found: []
   ✅ Keyword validation passed!

🎉 E2E Test Complete!
   ✅ Phase 1: Test isolation infrastructure working
   ✅ Phase 2: Real LLM integration successful
   ✅ Phase 3: Keyword validation passed
   📈 Overall score: 0.85/1.0
```

### Failed Test Output (Missing Keywords)

```
🚀 Running LLM E2E test with provider: anthropic
   Model: claude-3-sonnet-20240229

[... test setup ...]

🔍 Phase 3: Validating LLM response with keyword matching
   📊 Validation Results:
      • Score: 0.42
      • Required keywords found: ["Python"]
      • Optional keywords found: []
      • Forbidden keywords found: []
      • Missing required keywords: ["Django", "PostgreSQL", "GraphQL"]

❌ Test Failed!
   LLM response validation failed for provider anthropic: Tech Stack Analysis - Saleor

   📝 Full Response:
   This appears to be a Python project...

   📊 Validation Details:
   • Score: 0.42 (minimum: 0.70)
   • Required found: ["Python"]
   • Required missing: ["Django", "PostgreSQL", "GraphQL"]
   • Forbidden found: []
   • Optional found: []
```

### Failed Test Output (Tool Calls Not Working)

```
🚀 Running LLM E2E test with provider: openai
   Model: gpt-5-codex

[... test setup ...]

🎯 Phase 3: Testing LLM interaction with keyword validation
   📤 Prompt sent to AI session: What is the tech stack of this project?...
   ⏳ Waiting for AI session response...
   ⏳ Waiting for AI response... (attempt 1/48)
   ⏳ Waiting for AI response... (attempt 2/48)
   ...
   ⏳ Waiting for AI response... (attempt 48/48)
   ⚠️  Timeout waiting for AI response after 240 seconds

❌ Test Failed!
   No AI outputs generated

   Possible causes:
   1. Tool calls not working - LLM can't read files
   2. API error - check logs for error messages
   3. Session creation failed
```

---

## Appendix B: Quick Reference

### Provider URLs
- **OpenAI Chat**: `https://api.openai.com/v1/chat/completions`
- **OpenAI Responses**: `https://api.openai.com/v1/responses`
- **Anthropic**: `https://api.anthropic.com/v1/messages`
- **xAI/Grok**: `https://api.x.ai/v1/chat/completions`

### Tool Call Formats

**OpenAI (Chat Completions)**:
```json
{
  "message": {
    "role": "assistant",
    "tool_calls": [
      {
        "id": "call_abc123",
        "type": "function",
        "function": {
          "name": "read_file",
          "arguments": "{\"path\":\"manage.py\"}"
        }
      }
    ]
  }
}
```

**OpenAI (Responses API)**:
```json
{
  "output": [
    {
      "type": "function_call",
      "call_id": "call_abc123",
      "name": "read_file",
      "arguments": "{\"path\":\"manage.py\"}"
    }
  ]
}
```

**Anthropic**:
```json
{
  "content": [
    {
      "type": "tool_use",
      "id": "toolu_abc123",
      "name": "read_file",
      "input": {
        "path": "manage.py"
      }
    }
  ]
}
```

### Required Keywords by Test

**Saleor Tech Stack Test**:
- Required: Django, Python, PostgreSQL, GraphQL
- Optional: JavaScript, Node
- Minimum score: 0.7

---

**Document Version**: 1.0
**Last Updated**: 2025-01-05
**Related Documents**:
- `LLM_CLIENT_REFACTOR.md` - Technical architecture
- `manager/tests/llm_e2e_real_test.rs` - Test implementation
- `run_llm_e2e_test.sh` - Test runner script

**Status**: Ready for Implementation
