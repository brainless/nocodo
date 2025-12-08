# Test Runner

Compact test runner for nocodo-llm-sdk that conditionally runs integration tests based on available API keys.

## Usage

```bash
# From nocodo-llm-sdk directory
cargo run --bin test-runner --features test-runner -- test-config.toml
```

## Configuration

Create a `test-config.toml` file with your API keys:

```toml
[api_keys]
ANTHROPIC_API_KEY = "sk-ant-..."
OPENAI_API_KEY = "sk-..."
XAI_API_KEY = "xai-..."
CEREBRAS_API_KEY = "csk-..."
```

See `test-config.example.toml` for a template.

## Behavior

1. **Always runs**: Unit tests (no API keys needed)
2. **Conditionally runs**: Integration tests only if corresponding API key exists
3. **Always runs**: Zen integration tests (free, no API keys needed)

## Test Matrix

| Test | API Key Required | Description |
|------|------------------|-------------|
| Unit tests | None | All unit tests |
| `claude_integration` | `ANTHROPIC_API_KEY` | Claude API tests |
| `gpt_integration` | `OPENAI_API_KEY` | OpenAI GPT tests |
| `grok_integration` | `XAI_API_KEY` | xAI Grok tests |
| `glm_integration` | `CEREBRAS_API_KEY` | Cerebras GLM tests |
| `zen_grok_integration` | None (free) | Zen Grok tests |
| `zen_glm_integration` | None (free) | Zen GLM Big Pickle tests |

## Output

The runner uses `--quiet` mode to minimize noise. Only test results are shown.
Exit code 0 means all tests passed, non-zero means failures occurred.
