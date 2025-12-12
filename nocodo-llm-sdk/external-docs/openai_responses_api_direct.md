# OpenAI Responses API - Direct Access

## Overview

The Responses API is OpenAI's newest and most advanced API, designed as a successor to Chat Completions. It's specifically optimized for reasoning models and agentic workflows, providing a structured loop for reasoning and acting rather than a simple turn-based chat interface.

## Base URL

```
https://api.openai.com/v1/responses
```

## Authentication

Use your OpenAI API key in the Authorization header:

```bash
Authorization: Bearer YOUR_OPENAI_API_KEY
```

## Available Models

### GPT-5 Series
- `gpt-5.1-codex-max` - Best agentic coding model, ~30% fewer thinking tokens than gpt-5.1-codex
- `gpt-5.1-codex` - Optimized for long-running, agentic coding tasks
- `gpt-5.1-codex-mini` - Smaller, faster version of gpt-5.1-codex
- `gpt-5.1` - General purpose GPT-5.1 model
- `gpt-5.1-chat` - Conversational optimized version
- `gpt-5-pro` - Pro tier model
- `gpt-5` - Base GPT-5 model
- `gpt-5-mini` - Smaller GPT-5 variant
- `gpt-5-nano` - Smallest GPT-5 variant
- `gpt-5-chat` - Chat-optimized GPT-5

### GPT-4 Series
- `gpt-4o` - GPT-4 omni model
- `gpt-4o-mini` - Smaller GPT-4o variant
- `gpt-4.1` - Latest GPT-4 generation
- `gpt-4.1-mini` - Smaller GPT-4.1 variant
- `gpt-4.1-nano` - Smallest GPT-4.1 variant

### Reasoning Models
- `o1` - Reasoning model
- `o3-mini` - Small reasoning model
- `o3` - Mid-tier reasoning model
- `o4-mini` - Small reasoning model, latest generation

## Basic Usage

### Python with OpenAI SDK

```python
from openai import OpenAI

client = OpenAI(api_key="YOUR_OPENAI_API_KEY")

response = client.responses.create(
    model="gpt-5.1-codex",
    input="Write a Python function to calculate fibonacci numbers"
)

print(response.output_text)
```

### cURL

```bash
curl -X POST https://api.openai.com/v1/responses \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5.1-codex",
    "input": "Write a Python function to calculate fibonacci numbers"
  }'
```

## Key Architectural Features

### 1. Reasoning State Preservation

Unlike Chat Completions where reasoning is discarded between calls, the Responses API maintains the model's reasoning state across turns, improving performance by approximately 5% on TAUBench benchmarks.

### 2. Polymorphic Output Items

Rather than emitting a single message, Responses returns multiple output types in a clearly ordered sequence:
- Messages
- Tool calls
- Reasoning summaries
- Function calls

### 3. Hosted Tools

The API supports server-side tool execution, reducing round-trip costs:
- File Search
- Code Interpreter
- Web Search
- Image Generation
- Model Context Protocol (MCP)

## GPT-5.1-Codex Specific Features

### Built-in Coding Tools

The Codex models have native support for:

**apply_patch tool** - Optimized diff-based file editing (trained to excel at this diff format):
```python
response = client.responses.create(
    model="gpt-5.1-codex-max",
    tools=[
        {
            "type": "apply_patch",
            "description": "Apply code changes using diffs"
        }
    ],
    input="Refactor the authentication module to use async/await"
)
```

**shell_command** - Execute terminal operations with configurable timeouts:
```python
response = client.responses.create(
    model="gpt-5.1-codex",
    tools=[
        {
            "type": "shell_command",
            "timeout": 30
        }
    ],
    input="Run the test suite and fix any failing tests"
)
```

**update_plan** - Track task progress with status management:
```python
response = client.responses.create(
    model="gpt-5.1-codex-max",
    tools=[
        {
            "type": "update_plan"
        }
    ],
    input="Implement user authentication feature with tests"
)

# Task statuses: pending, in_progress, completed
```

## Streaming

```python
response = client.responses.create(
    model="gpt-5.1-codex",
    input="Explain how async/await works in Python",
    stream=True
)

for event in response:
    if event.type == 'response.output_text.delta':
        print(event.delta, end='')
```

## Chaining Responses

Maintain context across multiple interactions:

```python
# Initial response
response = client.responses.create(
    model="gpt-5.1-codex",
    input="Create a REST API for user management"
)

# Chain using previous_response_id
second_response = client.responses.create(
    model="gpt-5.1-codex",
    previous_response_id=response.id,
    input="Add authentication middleware to the API"
)
```

## Extended Caching

To use extended caching with GPT-5.1 models (saves costs on repeated prompts):

```python
response = client.responses.create(
    model="gpt-5.1-codex",
    input="Your prompt here",
    prompt_cache_retention="24h"  # Cache for 24 hours
)
```

## Response Format

```json
{
  "id": "resp_abc123",
  "created_at": 1741369938.0,
  "model": "gpt-5.1-codex",
  "object": "response",
  "output": [
    {
      "id": "msg_xyz789",
      "content": [
        {
          "annotations": [],
          "text": "Here's the fibonacci function...",
          "type": "output_text"
        }
      ],
      "role": "assistant",
      "type": "message"
    }
  ],
  "output_text": "Here's the fibonacci function...",
  "status": "completed",
  "usage": {
    "input_tokens": 15,
    "output_tokens": 250,
    "total_tokens": 265
  }
}
```

## Best Practices for GPT-5.1-Codex

### 1. Bias to Action
Default to implementing with reasonable assumptions rather than seeking clarification.

### 2. Parallel Tool Use
Batch parallel reads using `multi_tool_use.parallel` rather than sequential calls:

```python
response = client.responses.create(
    model="gpt-5.1-codex-max",
    tools=[{"type": "multi_tool_use.parallel"}],
    input="Read and analyze all test files in the project"
)
```

### 3. Semantic Tool Names
Make tool names and arguments as semantically correct as possible to improve model performance.

### 4. Tool Response Truncation
Limit output to 10k tokens, preserving the first and last halves with middle truncation.

### 5. Avoid Premature Communication
Remove instructions asking the model to "communicate an upfront plan, preambles, or other status updates" as these can cause premature termination. Instead, emphasize "autonomy and persistence" to ensure tasks complete end-to-end within single turns.

## Background Processing

For long-running tasks:

```python
# Start background task
response = client.responses.create(
    model="gpt-5.1-codex-max",
    input="Refactor the entire codebase to use TypeScript",
    background=True
)

print(response.status)  # "queued" or "in_progress"

# Poll for completion
import time
while response.status in {"queued", "in_progress"}:
    time.sleep(2)
    response = client.responses.retrieve(response.id)

print(f"Final status: {response.status}")
```

## Function Calling

```python
response = client.responses.create(
    model="gpt-5.1-codex",
    tools=[
        {
            "type": "function",
            "name": "run_tests",
            "description": "Run project tests",
            "parameters": {
                "type": "object",
                "properties": {
                    "test_path": {"type": "string"},
                    "verbose": {"type": "boolean"}
                },
                "required": ["test_path"]
            }
        }
    ],
    input="Run all unit tests and report failures"
)

# Handle tool calls in response.output
for output in response.output:
    if output.type == "function_call":
        # Execute the function
        result = run_tests(output.arguments["test_path"])

        # Send result back
        second_response = client.responses.create(
            model="gpt-5.1-codex",
            previous_response_id=response.id,
            input=[{
                "type": "function_call_output",
                "call_id": output.call_id,
                "output": result
            }]
        )
```

## Code Interpreter

```python
response = client.responses.create(
    model="gpt-5.1-codex",
    tools=[
        {
            "type": "code_interpreter",
            "container": {"type": "auto"}
        }
    ],
    instructions="Write and execute code to solve the problem",
    input="Analyze this CSV data and create visualizations"
)
```

## Model Context Protocol (MCP)

Connect to external tools and APIs:

```python
response = client.responses.create(
    model="gpt-5.1-codex",
    tools=[
        {
            "type": "mcp",
            "server_label": "github",
            "server_url": "https://api.github.com",
            "headers": {
                "Authorization": "Bearer YOUR_GITHUB_TOKEN"
            },
            "require_approval": "never"
        }
    ],
    input="List all open pull requests in the repository"
)
```

## Pricing

GPT-5.1 and gpt-5.1-chat-latest have the same pricing and rate limits as GPT-5. The gpt-5.1-codex-max model uses approximately 30% fewer thinking tokens than gpt-5.1-codex, which can result in cost savings for agentic coding tasks.

## Availability

- All GPT-5.1 models are available to developers on all paid tiers in the API
- GPT-5.1-Codex-Max is available in Codex CLI, IDE extension, cloud, and code review
- API access for GPT-5.1-Codex-Max is available now

## Installation

```bash
pip install --upgrade openai
```

## Response Status Values

- `completed`: Task finished successfully
- `queued`: Waiting to be processed (background mode)
- `in_progress`: Currently processing (background mode)
- `cancelled`: Task was cancelled
- `failed`: Task encountered an error

## Additional Resources

- Official API Reference: https://platform.openai.com/docs/api-reference/responses
- Quickstart Guide: https://platform.openai.com/docs/quickstart?api-mode=responses
- Migration Guide: https://platform.openai.com/docs/guides/migrate-to-responses
- GPT-5.1-Codex-Max Prompting Guide: https://cookbook.openai.com/examples/gpt-5/gpt-5-1-codex-max_prompting_guide

## Sources

Documentation compiled from:
- https://developers.openai.com/blog/responses-api/
- https://cookbook.openai.com/examples/gpt-5/gpt-5-1-codex-max_prompting_guide
- https://openai.com/index/gpt-5-1-for-developers/
- https://platform.openai.com/docs/models/gpt-5.1-codex
