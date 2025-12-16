# nocodo-workflow

Workflow orchestration for building AI agents with nocodo.

## Overview

`nocodo-workflow` provides the foundation for building AI agents through an interactive workflow. It helps users clearly define their agent requirements by:

1. Breaking down high-level goals into specific questions
2. Identifying required integrations (APIs, databases, files)
3. Collecting necessary credentials and configurations

## Features

- **Structured Workflow Responses**: Define agent requirements through JSON-structured conversations
- **Multi-Provider Support**: Works with multiple LLM providers (Claude, GPT, GLM, Grok)
- **Provider Preference Hierarchy**: Automatically selects the best available provider
- **Type-Safe**: Full Rust type safety with JSON schema validation

## Usage

### Basic Example

```rust
use nocodo_workflow::WorkflowResponse;

// Get the JSON schema to include in your system prompt
let schema = WorkflowResponse::json_schema();

// Parse LLM response
let response: WorkflowResponse = serde_json::from_str(&llm_output)?;

// Process questions and inputs
for question in response.questions {
    println!("Question: {}", question);
}

for input in response.inputs {
    println!("Need input: {} ({})", input.name, input.label);
}
```

### System Prompt Template

The LLM should be instructed to respond in this format:

```
You are a helpful assistant and are helping the user clearly define an agent.
You can respond only in JSON conforming to the given type.
You can ask questions for clarification, ask for data access (API, URL, DB, etc.).
```

## Testing

### Running Tests

The crate includes a test runner that automatically selects the best available LLM provider:

```bash
# Copy the example config
cp test-config.example.toml test-config.toml

# Add your API keys (optional - free providers work without keys)
# Edit test-config.toml

# Run tests using the test runner
cargo run --bin test-runner --features test-runner -- test-config.toml
```

### Provider Preference Hierarchy

The test runner selects providers in this order:

1. **z.ai/GLM 4.6** - z.ai GLM (requires `zai_api_key`, optionally set `zai_coding_plan = true`)
2. **zen/GLM 4.6** - Free Zen GLM (no API key required) - **DEFAULT**
3. **Anthropic/Claude Sonnet 4.5** - Claude (requires `anthropic_api_key`)

### Unit Tests

```bash
cargo test -p nocodo-workflow --lib
```

### Integration Tests

Integration tests are automatically run by the test runner. They validate:

- JSON response format
- Schema compliance
- Proper question/input generation

## Workflow Response Schema

```json
{
  "questions": ["string"],
  "inputs": [
    {
      "name": "string",
      "label": "string"
    }
  ]
}
```

### Fields

- `questions` - Array of clarifying questions the LLM wants to ask
- `inputs` - Array of required inputs (API keys, URLs, database names, etc.)
  - `name` - Identifier for the input (e.g., "api_key", "db_url")
  - `label` - Human-readable description of what this input is for

## Example Workflow

User input:
```
I would like an agent to collect available information about a company
before any call with anyone in that company that is scheduled in my calendar
```

LLM response:
```json
{
  "questions": [
    "Which calendar service do you use (Google Calendar, Outlook, etc.)?",
    "What sources should the agent use to collect company information?",
    "How far in advance should the agent prepare this information?"
  ],
  "inputs": [
    {
      "name": "calendar_api_key",
      "label": "API key for your calendar service"
    },
    {
      "name": "company_data_api",
      "label": "API endpoint for company information (e.g., Clearbit, LinkedIn)"
    }
  ]
}
```

## Dependencies

- `nocodo-llm-sdk` - LLM client library
- `serde` - Serialization/deserialization
- `schemars` - JSON schema generation

## License

MIT
