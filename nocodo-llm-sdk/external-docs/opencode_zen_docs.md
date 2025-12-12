# OpenCode Zen Documentation

## Overview

OpenCode Zen is "a curated list of models provided by OpenCode" that's currently in beta. It functions as an optional AI gateway providing access to tested and verified models for coding agents.

## Key Concepts

**Background & Purpose:**
The documentation explains that while many AI models exist, few work effectively as coding agents. OpenCode tested select models and collaborated with providers to ensure optimal performance, benchmarking combinations and creating a recommended list.

**How It Works:**
Users sign into OpenCode Zen, add billing details, copy their API key, connect via the `/connect` command in the TUI, and run `/models` to see recommendations. Charges are per-request with credit-based billing.

## API Endpoints

OpenCode Zen provides access through multiple API endpoints:

- **OpenAI-compatible (Chat Completions)**: `https://opencode.ai/zen/v1/chat/completions`
- **OpenAI Responses API**: `https://opencode.ai/zen/v1/responses`
- **Anthropic Messages API**: `https://opencode.ai/zen/v1/messages`
- **Models List**: `https://opencode.ai/zen/v1/models`

## Available Models

The platform provides access to models across multiple providers. Model IDs to use in API requests:

### OpenAI Models
- `gpt-5.1` - GPT 5.1
- `gpt-5.1-codex` - GPT 5.1 Codex
- `gpt-5.1-codex-max` - GPT 5.1 Codex Max
- `gpt-5` - GPT 5
- `gpt-5-codex` - GPT 5 Codex
- `gpt-5-nano` - GPT 5 Nano

### Anthropic Models
- `claude-sonnet-4-5` - Claude Sonnet 4.5
- `claude-sonnet-4` - Claude Sonnet 4
- `claude-haiku-4-5` - Claude Haiku 4.5
- `claude-3-5-haiku` - Claude 3.5 Haiku
- `claude-opus-4-5` - Claude Opus 4.5
- `claude-opus-4-1` - Claude Opus 4.1

### Google Models
- `gemini-3-pro` - Gemini 3 Pro

### Chinese Models
- `glm-4.6` - GLM 4.6
- `kimi-k2` - Kimi K2
- `kimi-k2-thinking` - Kimi K2 Thinking
- `qwen3-coder` - Qwen3 Coder 480B

### Free Models (No Auth Required)
- `grok-code` - Grok Code (free during beta)
- `big-pickle` - Big Pickle (free, limited time, routes to GLM-4.6)

### Other Models
- `alpha-gd4` - Alpha GD4

## Usage Examples

### Testing Free Models with curl

Free models (`grok-code` and `big-pickle`) can be used without authentication:

```bash
# Test grok-code model
curl -X POST https://opencode.ai/zen/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "grok-code",
    "messages": [{"role": "user", "content": "What is 2+2?"}],
    "max_tokens": 50
  }'

# Test big-pickle model
curl -X POST https://opencode.ai/zen/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "big-pickle",
    "messages": [{"role": "user", "content": "What is 2+2?"}],
    "max_tokens": 50
  }'
```

### Authenticated API Calls

For paid models, include your API key in the Authorization header:

```bash
curl -X POST https://opencode.ai/zen/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "model": "gpt-5.1",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

## Pricing Structure

Models use pay-as-you-go pricing per 1M tokens. Free models include Grok Code and Big Pickle (limited time). Claude Sonnet 4.5 costs "$3.00" input/$15.00 output for tokens under 200K.

## Privacy & Data Retention

Providers follow zero-retention policies except OpenAI (30 days) and Anthropic (30 days) per their respective data policies. Free models may use data for improvement during beta.

## Team Features

Teams receive free workspace management during beta, with role-based access (Admin/Member), model curation controls, and support for bring-your-own-key integration with OpenAI or Anthropic.
