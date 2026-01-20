# Add Structured JSON Agent

## Overview

Create an agent that constrains LLM responses to JSON conforming to specified TypeScript types from the `shared-types` crate. This enables type-safe, structured output from LLMs for specific domains (project management, workflows, etc.).

## Prerequisites

**IMPORTANT**: This task depends on the TypeScript generation function from `shared-types/tasks/add-typescript-generation-function.md`.

Before implementing this agent:
1. Complete the `add-typescript-generation-function` task in `shared-types`
2. Verify the function exists: `shared_types::generate_typescript_definitions()`
3. Ensure tests pass: `cd shared-types && cargo test typescript_gen`

## Objectives

Create a new agent `StructuredJsonAgent` that:
- Accepts a list of type names from `shared-types` at initialization
- Generates TypeScript definitions using `shared_types::generate_typescript_definitions()`
- Includes type definitions in the system prompt
- Validates LLM responses conform to requested types
- Returns only valid JSON matching the specified schema

## Architecture

### Component Structure

```
nocodo-agents/
├── src/
│   ├── structured_json/
│   │   ├── mod.rs          # Main agent implementation
│   │   ├── validator.rs    # JSON schema validation
│   │   └── tests.rs        # Unit tests
│   └── lib.rs              # Add structured_json module
└── bin/
    └── structured_json_runner.rs  # CLI runner for the agent
```

**Dependencies**:
- `shared-types` crate provides `generate_typescript_definitions()` function

### Agent Configuration

The agent should accept configuration at creation time:

```rust
pub struct StructuredJsonAgentConfig {
    /// List of type names from shared-types to include
    pub type_names: Vec<String>,
    /// Domain description for the agent
    pub domain_description: String,
}
```

Example usage:
```rust
let config = StructuredJsonAgentConfig {
    type_names: vec![
        "PMProject".to_string(),
        "Workflow".to_string(),
        "WorkflowStep".to_string(),
        "WorkflowWithSteps".to_string(),
    ],
    domain_description: "Project management and workflow planning".to_string(),
};

let agent = StructuredJsonAgent::new(client, database, tool_executor, config)?;
```

### System Prompt Design

The agent's system prompt should:

1. **Define the role**: Clearly state the agent responds only in structured JSON
2. **Include TypeScript definitions**: Embed the requested type definitions
3. **Specify validation rules**: Explain the response must validate against the types
4. **Provide examples**: Show sample JSON responses for clarity

Example structure:
```
You are a specialized AI assistant that responds exclusively in structured JSON.

Your responses must conform to one or more of the following TypeScript types:

<TYPE_DEFINITIONS>
{generated_typescript_definitions}
</TYPE_DEFINITIONS>

IMPORTANT RULES:
1. Your entire response must be valid JSON
2. The JSON must match one of the provided TypeScript types exactly
3. Do not include any text outside the JSON structure
4. All required fields must be present
5. Field types must match exactly (string, number, boolean, etc.)

{domain_description}
```

### JSON Validation Flow

```
User Prompt
    ↓
LLM Response (should be JSON)
    ↓
Extract JSON from response
    ↓
Validate against TypeScript schema
    ↓
If valid: Return JSON
If invalid: Ask LLM to correct (max 3 retries)
```

## Implementation Steps

### Phase 1: JSON Validator Module

**File**: `nocodo-agents/src/structured_json/validator.rs`

Implement JSON schema validation:

```rust
use schemars::JsonSchema;
use serde_json::Value;

pub struct TypeValidator {
    type_definitions: Vec<String>,
    schemas: Vec<Value>,
}

impl TypeValidator {
    pub fn new(type_definitions: Vec<String>) -> Result<Self, anyhow::Error> {
        // Parse TypeScript definitions
        // Generate JSON schemas from TS types
        // Store for validation
    }

    pub fn validate(&self, json: &Value) -> Result<(), ValidationError> {
        // Validate JSON against any of the stored schemas
        // Return detailed error if validation fails
    }

    pub fn get_type_definitions(&self) -> String {
        // Return formatted TypeScript definitions for prompt
    }
}

#[derive(Debug)]
pub struct ValidationError {
    pub message: String,
    pub expected_types: Vec<String>,
}
```

**Approach**:
- Use `schemars` to generate JSON schemas from Rust types
- Validate using `jsonschema` crate or similar
- Provide clear error messages for LLM to self-correct

**Dependencies to add**:
```toml
schemars = { version = "0.8", features = ["preserve_order"] }
jsonschema = "0.18"
```

### Phase 2: StructuredJsonAgent Implementation

**File**: `nocodo-agents/src/structured_json/mod.rs`

```rust
use crate::{database::Database, Agent, AgentTool};
use async_trait::async_trait;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use std::sync::Arc;

mod validator;
use validator::TypeValidator;

pub struct StructuredJsonAgentConfig {
    pub type_names: Vec<String>,
    pub domain_description: String,
}

pub struct StructuredJsonAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
    validator: TypeValidator,
    system_prompt: String,
    config: StructuredJsonAgentConfig,
}

impl StructuredJsonAgent {
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        config: StructuredJsonAgentConfig,
    ) -> anyhow::Result<Self> {
        // 1. Generate TypeScript definitions using shared-types function
        let type_names: Vec<&str> = config.type_names.iter().map(|s| s.as_str()).collect();
        let type_definitions = shared_types::generate_typescript_definitions(&type_names)
            .map_err(|e| anyhow::anyhow!("Failed to generate TypeScript definitions: {}", e))?;

        // 2. Create validator
        let validator = TypeValidator::new(type_definitions.clone())?;

        // 3. Generate system prompt with type definitions
        let system_prompt = Self::generate_system_prompt(
            &type_definitions,
            &config.domain_description,
        );

        Ok(Self {
            client,
            database,
            tool_executor,
            validator,
            system_prompt,
            config,
        })
    }

    fn generate_system_prompt(type_defs: &str, domain_desc: &str) -> String {
        format!(
            r#"You are a specialized AI assistant that responds exclusively in structured JSON.

Your responses must conform to one or more of the following TypeScript types:

<TYPE_DEFINITIONS>
{type_defs}
</TYPE_DEFINITIONS>

IMPORTANT RULES:
1. Your entire response must be valid JSON
2. The JSON must match one of the provided TypeScript types exactly
3. Do not include any text outside the JSON structure
4. All required fields must be present
5. Field types must match exactly (string, number, boolean, etc.)
6. Use proper JSON formatting (double quotes, no trailing commas, etc.)

Domain: {domain_desc}

When responding:
- Analyze the user's request
- Determine which type(s) best represent the response
- Generate valid JSON matching those types
- Include all required fields with appropriate values
"#
        )
    }

    async fn validate_and_retry(
        &self,
        session_id: i64,
        max_retries: u32,
    ) -> anyhow::Result<serde_json::Value> {
        // Execute LLM call
        // Extract JSON from response
        // Validate
        // If invalid, provide feedback and retry
        todo!()
    }
}

#[async_trait]
impl Agent for StructuredJsonAgent {
    fn objective(&self) -> &str {
        "Generate structured JSON responses conforming to specified types"
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn tools(&self) -> Vec<AgentTool> {
        // This agent doesn't need tools - pure JSON generation
        vec![]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        // 1. Create user message in database
        self.database.create_message(session_id, "user", user_prompt)?;

        // 2. Execute with validation and retry logic
        let json_value = self.validate_and_retry(session_id, 3).await?;

        // 3. Return formatted JSON
        Ok(serde_json::to_string_pretty(&json_value)?)
    }
}
```

### Phase 3: Binary Runner

**File**: `nocodo-agents/bin/structured_json_runner.rs`

```rust
use nocodo_agents::factory::AgentFactory;
use nocodo_agents::structured_json::{StructuredJsonAgent, StructuredJsonAgentConfig};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    prompt: String,

    #[arg(short, long)]
    types: Vec<String>,

    #[arg(short, long, default_value = "Structured data generation")]
    domain: String,

    #[arg(short, long)]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse args
    // Initialize LLM client, database, tool executor
    // Create StructuredJsonAgentConfig
    // Create agent
    // Execute
    // Print result
}
```

Add to `Cargo.toml`:
```toml
[[bin]]
name = "structured-json-runner"
path = "bin/structured_json_runner.rs"
```

## Testing Strategy

### Unit Tests

**File**: `nocodo-agents/src/structured_json/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_validator_valid_json() {
        // Test validation with correct JSON
    }

    #[test]
    fn test_type_validator_invalid_json() {
        // Test validation fails with incorrect JSON
    }

    #[tokio::test]
    async fn test_agent_workflow_generation() {
        // Test with mock LLM that returns valid workflow JSON
    }

    #[tokio::test]
    async fn test_agent_retry_on_invalid() {
        // Test retry logic when LLM returns invalid JSON
    }
}
```

### Integration Tests

Create test that:
1. Generates all types with new binary
2. Creates agent with specific types
3. Sends prompts and validates responses
4. Tests error handling and retry logic

## API Integration

Update `shared-types/src/agent.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StructuredJsonAgentConfig {
    pub type_names: Vec<String>,
    pub domain_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum AgentConfig {
    CodebaseAnalysis(CodebaseAnalysisAgentConfig),
    Sqlite(SqliteAgentConfig),
    Tesseract(TesseractAgentConfig),
    StructuredJson(StructuredJsonAgentConfig),  // NEW
}
```

Update factory in `nocodo-agents/src/factory.rs`:

```rust
pub enum AgentType {
    CodebaseAnalysis,
    Tesseract,
    StructuredJson,  // NEW
}

pub fn create_agent_with_tools(
    agent_type: AgentType,
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
    config: Option<AgentConfig>,
) -> Box<dyn Agent> {
    match agent_type {
        AgentType::StructuredJson => {
            // Extract config
            let json_config = match config {
                Some(AgentConfig::StructuredJson(c)) => c,
                _ => panic!("StructuredJson config required"),
            };
            Box::new(StructuredJsonAgent::new(client, database, tool_executor, json_config).unwrap())
        }
        // ... other agents
    }
}
```

## Usage Examples

### Example 1: Workflow Generation

```bash
cargo run --bin structured-json-runner -- \
  --types Workflow WorkflowWithSteps WorkflowStep \
  --domain "Software project workflow planning" \
  --prompt "Create a workflow for deploying a web application with testing and staging steps" \
  --config config.toml
```

Expected output:
```json
{
  "workflow": {
    "id": 1,
    "project_id": 1,
    "name": "Web Application Deployment",
    "parent_workflow_id": null,
    "branch_condition": null,
    "created_at": 1234567890
  },
  "steps": [
    {
      "id": 1,
      "workflow_id": 1,
      "step_number": 1,
      "description": "Run unit tests",
      "created_at": 1234567890
    },
    {
      "id": 2,
      "workflow_id": 1,
      "step_number": 2,
      "description": "Deploy to staging environment",
      "created_at": 1234567890
    }
  ]
}
```

### Example 2: Project Planning

```bash
cargo run --bin structured-json-runner -- \
  --types PMProject \
  --domain "Project management" \
  --prompt "Create a project structure for an e-commerce mobile app" \
  --config config.toml
```

Expected output:
```json
{
  "id": 1,
  "name": "E-commerce Mobile App",
  "description": "A mobile application for online shopping with payment integration",
  "created_at": 1234567890
}
```

## Benefits

1. **Type Safety**: Responses are guaranteed to match defined schemas
2. **Validation**: Automatic validation prevents malformed data
3. **Flexibility**: Configure which types to use per session
4. **Reusability**: Same agent works for different domains (PM, auth, workflows)
5. **Self-Correction**: Retry logic allows LLM to fix invalid responses
6. **Developer Experience**: Clear error messages when validation fails

## Future Enhancements

1. **Dynamic Type Loading**: Load types at runtime from a registry
2. **Type Composition**: Allow combining multiple types in responses
3. **Partial Validation**: Validate parts of complex nested structures
4. **Schema Evolution**: Handle version mismatches gracefully
5. **Streaming Support**: Validate JSON as it's generated
6. **Custom Validators**: Allow domain-specific validation rules
7. **Type Inference**: Suggest types based on user prompt

## Dependencies

Add to `nocodo-agents/Cargo.toml`:
```toml
[dependencies]
# Existing dependencies...
shared-types = { path = "../shared-types" }  # Already exists, just ensure it's there
jsonschema = "0.18"

[dev-dependencies]
# Existing dev-dependencies...
```

**Note**: The `shared-types` dependency provides `generate_typescript_definitions()` function (see prerequisite task).

## Acceptance Criteria

- [ ] `shared_types::generate_typescript_definitions()` function is available (from prerequisite task)
- [ ] `StructuredJsonAgent` accepts configuration with type names
- [ ] Agent calls `generate_typescript_definitions()` with requested types
- [ ] Agent system prompt includes generated TypeScript type definitions
- [ ] Agent validates LLM responses against specified types
- [ ] Invalid responses trigger retry with error feedback (max 3 retries)
- [ ] Agent returns only valid JSON matching requested types
- [ ] Clear error messages when unknown types are requested
- [ ] Binary runner `structured-json-runner` works end-to-end
- [ ] Unit tests cover validation logic
- [ ] Integration test demonstrates full workflow with library function
- [ ] Documentation includes usage examples and prerequisites
- [ ] AgentConfig enum updated with StructuredJson variant
- [ ] No file system dependencies (all in-memory)

## Benefits of Library Function Approach

1. **No file system**: Pure in-memory, no I/O overhead
2. **Always current**: Types always match code, no sync issues
3. **Simpler**: Just call `generate_typescript_definitions()`
4. **Faster**: No file reading, instant generation
5. **Better errors**: Compile-time verification
6. **Easier testing**: Mock and test the function directly

## References

- Prerequisite task: `shared-types/tasks/add-typescript-generation-function.md`
- Existing agents: `nocodo-agents/src/sqlite_reader/mod.rs`
- Agent trait: `nocodo-agents/src/lib.rs`
- Tool schemas: `nocodo-agents/src/tools/llm_schemas.rs`
