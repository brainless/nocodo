# OpenAI Chat Completions API Reference

**API Type**: Chat Completions (Traditional, Stateless)
**Status**: Stable, widely adopted
**Sources**:
- [OpenAI Chat Completions API Reference](https://platform.openai.com/docs/api-reference/chat)
- [Azure OpenAI Chat Completions Reference](https://learn.microsoft.com/en-us/azure/ai-foundry/openai/reference)

**Last Updated**: December 2025

---

## Overview

The Chat Completions API generates model responses from a list of messages comprising a conversation. It's the traditional, stateless API for interacting with OpenAI's language models including GPT-5, GPT-4, and GPT-3.5.

**Key Characteristics:**
- ✅ Stateless - no server-side conversation storage
- ✅ Simple request/response pattern
- ✅ Compatible with existing patterns (GLM, Grok)
- ✅ Supports streaming, function calling, vision
- ✅ Well-established with extensive examples

---

## API Endpoint

### OpenAI
```
POST https://api.openai.com/v1/chat/completions
```

### Azure OpenAI
```
POST https://{your-resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version=2024-10-21
```

---

## Authentication

### OpenAI
```
Authorization: Bearer YOUR_OPENAI_API_KEY
Content-Type: application/json
```

### Azure OpenAI
```
api-key: YOUR_AZURE_API_KEY
Content-Type: application/json
```

Or with Microsoft Entra ID:
```
Authorization: Bearer YOUR_AUTH_TOKEN
Content-Type: application/json
```

---

## Request Parameters

### Required Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `messages` | array | Array of message objects (system, user, assistant, tool) |

### Core Optional Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `model` | string | — | Model ID (e.g., "gpt-5", "gpt-5-codex", "gpt-4o") |
| `temperature` | number | 1.0 | Sampling temperature 0-2. Higher = random, Lower = deterministic |
| `top_p` | number | 1.0 | Nucleus sampling 0-1. Alternative to temperature |
| `max_tokens` | integer | — | **Legacy**: Max tokens in completion |
| `max_completion_tokens` | integer | — | **Recommended**: Upper bound for generated tokens (includes reasoning) |
| `n` | integer | 1 | Number of completion choices to generate |
| `stream` | boolean | false | Enable streaming with server-sent events |
| `stop` | string or array | null | Up to 4 sequences where API stops generating |
| `presence_penalty` | number | 0 | -2.0 to 2.0. Penalize tokens based on presence |
| `frequency_penalty` | number | 0 | -2.0 to 2.0. Penalize tokens based on frequency |
| `logit_bias` | object | null | Modify likelihood of specific tokens (-100 to 100) |
| `user` | string | — | Unique end-user ID for monitoring |
| `seed` | integer | — | For deterministic sampling (best-effort) |

### Advanced Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `response_format` | object | Output format: `{"type": "text"}`, `{"type": "json_object"}`, or JSON schema |
| `tools` | array | List of tools/functions the model may call (max 128) |
| `tool_choice` | string/object | Control tool calling: `"none"`, `"auto"`, `"required"`, or specific function |
| `parallel_tool_calls` | boolean | Enable parallel function execution (default: true) |
| `logprobs` | boolean | Return log probabilities of output tokens |
| `top_logprobs` | integer | Number of most likely tokens (0-20) to return per position |

### GPT-5 Specific Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `reasoning_effort` | string | **GPT-5 only**: `"minimal"`, `"low"`, `"medium"` (default), `"high"` |

---

## Message Format

### Message Object Structure

```typescript
interface Message {
  role: "system" | "user" | "assistant" | "tool";
  content: string | ContentPart[];  // string or array of content parts
  name?: string;                     // optional participant name
  tool_calls?: ToolCall[];           // for assistant messages with function calls
  tool_call_id?: string;             // for tool response messages
}
```

### Message Types

#### 1. System Message
```json
{
  "role": "system",
  "content": "You are a helpful assistant that explains technical concepts simply."
}
```

#### 2. User Message (Text)
```json
{
  "role": "user",
  "content": "Explain quantum computing in simple terms"
}
```

#### 3. User Message (Vision - Text + Image)
```json
{
  "role": "user",
  "content": [
    {
      "type": "text",
      "text": "What's in this image?"
    },
    {
      "type": "image_url",
      "image_url": {
        "url": "https://example.com/image.jpg",
        "detail": "high"  // "low", "high", or "auto"
      }
    }
  ]
}
```

#### 4. Assistant Message (Text)
```json
{
  "role": "assistant",
  "content": "Quantum computing uses quantum mechanics principles..."
}
```

#### 5. Assistant Message (Function Call)
```json
{
  "role": "assistant",
  "content": null,
  "tool_calls": [
    {
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "get_weather",
        "arguments": "{\"location\": \"New York\"}"
      }
    }
  ]
}
```

#### 6. Tool Response Message
```json
{
  "role": "tool",
  "tool_call_id": "call_abc123",
  "content": "The weather in New York is sunny, 72°F"
}
```

---

## Response Structure

### Standard Response (Non-Streaming)

```json
{
  "id": "chatcmpl-7R1nGnsXO8n4oi9UPz2f3UHdgAYMn",
  "object": "chat.completion",
  "created": 1686676106,
  "model": "gpt-5",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Quantum computing leverages quantum mechanics..."
      },
      "finish_reason": "stop",
      "logprobs": null
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 150,
    "total_tokens": 170,
    "completion_tokens_details": {
      "reasoning_tokens": 0
    }
  },
  "system_fingerprint": "fp_1234567890abcdef"
}
```

### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique completion identifier |
| `object` | string | Always `"chat.completion"` |
| `created` | integer | Unix timestamp |
| `model` | string | Model used |
| `choices` | array | Array of completion choices |
| `usage` | object | Token usage statistics |
| `system_fingerprint` | string | Backend configuration fingerprint (for reproducibility) |

### Choice Object

| Field | Type | Description |
|-------|------|-------------|
| `index` | integer | Choice index |
| `message` | object | The generated message |
| `finish_reason` | string | Why stopped: `"stop"`, `"length"`, `"tool_calls"`, `"content_filter"` |
| `logprobs` | object | Log probability information (if requested) |

### Usage Object

| Field | Type | Description |
|-------|------|-------------|
| `prompt_tokens` | integer | Tokens in prompt |
| `completion_tokens` | integer | Tokens in completion |
| `total_tokens` | integer | Total tokens used |
| `completion_tokens_details` | object | Breakdown with `reasoning_tokens` (GPT-5) |

---

## Streaming Response

When `stream: true`, responses are sent as Server-Sent Events (SSE):

### Stream Format
```
data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1686676106,"model":"gpt-5","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1686676106,"model":"gpt-5","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1686676106,"model":"gpt-5","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1686676106,"model":"gpt-5","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

### Stream Chunk Object

```json
{
  "id": "chatcmpl-xxx",
  "object": "chat.completion.chunk",
  "created": 1686676106,
  "model": "gpt-5",
  "choices": [
    {
      "index": 0,
      "delta": {
        "role": "assistant",
        "content": "text delta"
      },
      "finish_reason": null
    }
  ]
}
```

---

## Supported Models (December 2025)

### GPT-5 Series (Latest)
- **`gpt-5`** - Latest GPT-5 model (recommended for tracking latest)
- **`gpt-5-2025-08-07`** - Dated snapshot (for reproducibility)
- **`gpt-5-mini`** - Smaller, faster, cost-effective
- **`gpt-5-nano`** - Smallest, fastest
- **`gpt-5-chat`** - Chat-optimized
- **`gpt-5-codex`** - Code-specialized (2025-09-11)

### GPT-5.1 Series (November 2025)
- **`gpt-5.1`** - Latest flagship
- **`gpt-5.1-chat`** - Chat-optimized
- **`gpt-5.1-codex`** - Code-specialized
- **`gpt-5.1-codex-mini`** - Smaller code variant
- **`gpt-5.1-codex-max`** - Maximum capability (2025-12-04)

### GPT-4 Series
- **`gpt-4o`** - GPT-4 optimized (2024-11-20, 2024-08-06, 2024-05-13)
- **`gpt-4o-mini`** - Smaller GPT-4 (2024-07-18)
- **`gpt-4-turbo`** - Turbo variant
- **`gpt-4`** - Original GPT-4

### GPT-3.5 Series
- **`gpt-3.5-turbo`** - Legacy, cost-effective

### Token Limits

**GPT-5 Models:**
- Context: Up to 272,000 input tokens
- Output: Up to 128,000 tokens
- Total: 400,000 tokens

---

## Example Requests

### 1. Basic Text Completion

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5",
    "messages": [
      {
        "role": "system",
        "content": "You are a helpful assistant."
      },
      {
        "role": "user",
        "content": "Explain quantum computing in simple terms"
      }
    ],
    "temperature": 0.7,
    "max_completion_tokens": 500
  }'
```

### 2. Multi-turn Conversation

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "What is photosynthesis?"},
      {"role": "assistant", "content": "Photosynthesis is the process..."},
      {"role": "user", "content": "Explain it for a 5-year-old"}
    ],
    "temperature": 0.7
  }'
```

### 3. With Vision (Image Analysis)

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {
        "role": "user",
        "content": [
          {
            "type": "text",
            "text": "What is in this image?"
          },
          {
            "type": "image_url",
            "image_url": {
              "url": "https://example.com/image.jpg",
              "detail": "high"
            }
          }
        ]
      }
    ],
    "max_tokens": 1024
  }'
```

### 4. With Reasoning Effort (GPT-5)

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5",
    "messages": [
      {
        "role": "user",
        "content": "Solve this complex math problem: ..."
      }
    ],
    "reasoning_effort": "high",
    "max_completion_tokens": 2000
  }'
```

### 5. With Function Calling

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5",
    "messages": [
      {
        "role": "user",
        "content": "What is the weather in New York?"
      }
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "get_weather",
          "description": "Get current weather for a location",
          "parameters": {
            "type": "object",
            "properties": {
              "location": {
                "type": "string",
                "description": "City name"
              }
            },
            "required": ["location"]
          }
        }
      }
    ],
    "tool_choice": "auto"
  }'
```

### 6. With Streaming

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5",
    "messages": [
      {
        "role": "user",
        "content": "Write a short poem about coding"
      }
    ],
    "stream": true,
    "max_tokens": 200
  }'
```

### 7. With JSON Mode

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5",
    "messages": [
      {
        "role": "user",
        "content": "Extract name and age from: John is 30 years old"
      }
    ],
    "response_format": {
      "type": "json_object"
    }
  }'
```

### 8. With Deterministic Output (Seed)

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5",
    "messages": [
      {"role": "user", "content": "Generate a random name"}
    ],
    "seed": 42,
    "temperature": 0.7
  }'
```

---

## Error Handling

### Error Response Structure

```json
{
  "error": {
    "message": "Invalid API key provided",
    "type": "invalid_request_error",
    "param": null,
    "code": "invalid_api_key"
  }
}
```

### Common Error Codes

| HTTP Status | Error Type | Description |
|-------------|------------|-------------|
| 400 | `invalid_request_error` | Malformed request |
| 401 | `authentication_error` | Invalid API key |
| 403 | `permission_error` | Access denied |
| 404 | `not_found_error` | Resource not found |
| 429 | `rate_limit_exceeded` | Too many requests |
| 500 | `server_error` | Internal server error |
| 503 | `service_unavailable` | Service temporarily unavailable |

### Rate Limit Headers

```
x-ratelimit-limit-requests: 10000
x-ratelimit-limit-tokens: 2000000
x-ratelimit-remaining-requests: 9999
x-ratelimit-remaining-tokens: 1999950
x-ratelimit-reset-requests: 8.64s
x-ratelimit-reset-tokens: 432ms
```

---

## Implementation Notes

### For SDK Development

1. **Base URL**: `https://api.openai.com/v1`
2. **Endpoint**: `/chat/completions`
3. **Auth Header**: `Authorization: Bearer {api_key}`
4. **Content-Type**: `application/json`
5. **Timeout**: Recommended 5 minutes (300s) for long completions

### Key Differences from Other Providers

| Feature | OpenAI | GLM (Cerebras) | Grok (xAI) |
|---------|--------|----------------|------------|
| **Base URL** | `api.openai.com` | `api.cerebras.ai` | `api.x.ai` |
| **Auth Header** | `Authorization: Bearer` | `Authorization: Bearer` | `Authorization: Bearer` |
| **Token Param** | `max_completion_tokens` | `max_completion_tokens` | `max_tokens` |
| **Messages Field** | `messages` | `messages` | `messages` |
| **Reasoning** | GPT-5: `reasoning_effort` | GLM: in response | N/A |
| **Stream Format** | SSE standard | SSE standard | SSE standard |

### Best Practices

1. **Use `max_completion_tokens`** instead of deprecated `max_tokens`
2. **Pin model versions** for reproducibility (e.g., `gpt-5-2025-08-07`)
3. **Use `seed` + `system_fingerprint`** for deterministic outputs
4. **Handle `finish_reason`** appropriately:
   - `"stop"` - Normal completion
   - `"length"` - Hit token limit
   - `"tool_calls"` - Function calling triggered
   - `"content_filter"` - Content policy violation
5. **Monitor `usage` object** for cost tracking
6. **Implement exponential backoff** for rate limits
7. **Handle streaming gracefully** - expect `[DONE]` event

---

## Comparison: Chat Completions vs Responses API

| Aspect | Chat Completions API ✅ | Responses API |
|--------|------------------------|---------------|
| **Complexity** | Simple, stateless | Complex, stateful |
| **State** | Manual conversation history | Server-side with `previous_response_id` |
| **Token Param** | `max_completion_tokens` | `max_output_tokens` |
| **Input** | `messages` array | `input` (string or array) |
| **Tools** | `tools` array (functions) | `tools` (functions, code_interpreter, MCP) |
| **Streaming** | SSE standard | SSE with richer events |
| **Best For** | Simple chat, compatibility | Advanced features, stateful apps |
| **SDK Pattern Match** | ✅ Perfect (GLM/Grok style) | ❌ Different pattern |

---

## Recommendation for nocodo-llm-sdk

**Use Chat Completions API** because:

1. ✅ **Matches existing pattern** - Same structure as GLM, Grok
2. ✅ **Simpler integration** - Stateless, straightforward
3. ✅ **v0.1 goals aligned** - Basic completions, no streaming yet
4. ✅ **Well-established** - Proven, stable API
5. ✅ **Easy maintenance** - Consistent with other providers

---

## Sources

- [OpenAI Chat Completions API Reference](https://platform.openai.com/docs/api-reference/chat)
- [OpenAI API Reference Introduction](https://platform.openai.com/docs/api-reference/introduction)
- [OpenAI Chat Completions Guide](https://platform.openai.com/docs/guides/chat-completions)
- [Azure OpenAI Chat Completions Reference](https://learn.microsoft.com/en-us/azure/ai-foundry/openai/reference)
- [OpenAI GPT-5 Developer Guide](https://openai.com/index/introducing-gpt-5-for-developers/)
- [OpenAI GPT-5.1 Announcement](https://openai.com/index/gpt-5-1-for-developers/)
