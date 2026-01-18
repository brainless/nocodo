# Per-Model Defaults Implementation

## Overview

The SDK now supports automatic application of per-model defaults for `max_output_tokens`, `temperature`, and `thinking_level` (for reasoning models like Gemini 3). These defaults are automatically applied when you specify a model, eliminating the need to manually configure these parameters for optimal performance.

## Implementation Details

### 1. Model Metadata Enhancement

Added `default_thinking_level` field to `ModelMetadata` struct:

```rust
pub struct ModelMetadata {
    // ... existing fields
    pub default_thinking_level: Option<&'static str>,
}
```

### 2. Gemini 3 Defaults

#### Gemini 3 Pro
- `default_temperature`: `1.0` (per Gemini docs recommendation)
- `default_max_tokens`: `1024` (sufficient for response + thinking tokens)
- `default_thinking_level`: `"high"` (default per Gemini docs)

#### Gemini 3 Flash
- `default_temperature`: `1.0`
- `default_max_tokens`: `512` (can be lower due to less thinking overhead)
- `default_thinking_level`: `"medium"` (balance of speed and quality)

### 3. Automatic Application in Builder

The Gemini `MessageBuilder::model()` method now:
1. Looks up model metadata by ID
2. Applies defaults only if values haven't been explicitly set
3. Respects user overrides

```rust
pub fn model(mut self, model: impl Into<String>) -> Self {
    let model_id = model.into();

    // Apply model-specific defaults from metadata
    if let Some(metadata) = crate::model_metadata::get_model_metadata(&model_id) {
        // Apply defaults only if not already set
        if self.generation_config.temperature.is_none() {
            self.generation_config.temperature = metadata.default_temperature;
        }

        if self.generation_config.max_output_tokens.is_none() {
            self.generation_config.max_output_tokens = metadata.default_max_tokens;
        }

        if self.generation_config.thinking_config.is_none() {
            if let Some(default_level) = metadata.default_thinking_level {
                self.generation_config.thinking_config = Some(ThinkingConfig {
                    thinking_level: default_level.to_string(),
                });
            }
        }
    }

    self.model = Some(model_id);
    self
}
```

## Usage

### Before (Manual Configuration Required)

```rust
let response = client
    .message_builder()
    .model(GEMINI_3_PRO)
    .max_output_tokens(1024)  // Had to specify
    .temperature(1.0)          // Had to specify
    .thinking_level("high")    // Had to specify
    .user_message("Hello")
    .send()
    .await?;
```

### After (Defaults Applied Automatically)

```rust
let response = client
    .message_builder()
    .model(GEMINI_3_PRO)  // Defaults automatically applied!
    .user_message("Hello")
    .send()
    .await?;
```

### Override Defaults When Needed

```rust
let response = client
    .message_builder()
    .model(GEMINI_3_PRO)
    .thinking_level("low")      // Override for faster response
    .max_output_tokens(256)      // Override for shorter output
    .user_message("Quick question")
    .send()
    .await?;
```

## Why This Matters

### Gemini 3 Pro Thinking Tokens

Gemini 3 Pro uses "dynamic thinking" which consumes tokens for internal reasoning **before** generating output text. Our testing revealed:

- With `max_output_tokens: 50`: Used 97 tokens for thinking, 0 for output → **FAILED**
- With `max_output_tokens: 100`: Used 97 tokens for thinking, 0 for output → **FAILED**
- With `max_output_tokens: 1000`: Used 128 tokens for thinking, 1 for output → **SUCCESS**

The default of 1024 tokens ensures Gemini 3 Pro has sufficient budget for both thinking and response generation.

### Temperature Recommendation

Per Gemini documentation:
> For Gemini 3, we strongly recommend keeping the temperature parameter at its default value of 1.0. Changing the temperature may lead to unexpected behavior, such as looping or degraded performance.

The SDK now enforces this by default.

## Benefits

1. **Better Defaults**: Models work optimally out-of-the-box
2. **Less Code**: No need to manually configure common parameters
3. **Fewer Errors**: Prevents token budget issues with reasoning models
4. **Best Practices**: Follows provider recommendations automatically
5. **Still Flexible**: Can override any default when needed

## Test Results

All Gemini integration tests now pass:

```bash
running 4 tests
test test_gemini_3_flash_with_thinking_level ... ok
test test_gemini_multi_turn_conversation ... ok
test test_gemini_3_pro_simple_completion ... ok
test test_gemini_with_system_instruction ... ok

test result: ok. 4 passed; 0 failed
```

## Future Considerations

This pattern can be extended to:
- Other providers (Claude, GPT-5, etc.) when they support reasoning
- Additional model-specific parameters
- Region-specific defaults
- Cost-optimized vs quality-optimized presets
