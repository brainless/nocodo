//! Model constants for supported LLM providers
//!
//! This module contains official model IDs and human-readable names for all supported providers.
//! Model IDs are sourced from official provider documentation.

/// Claude model constants
pub mod claude {
    /// Claude Sonnet 4.5 - Smart model for complex agents and coding
    /// Released: 2025-09-29
    pub const SONNET_4_5_ID: &str = "claude-sonnet-4-5-20250929";
    pub const SONNET_4_5_NAME: &str = "Claude Sonnet 4.5";

    /// Claude Haiku 4.5 - Fastest model with near-frontier intelligence
    /// Released: 2025-10-01
    pub const HAIKU_4_5_ID: &str = "claude-haiku-4-5-20251001";
    pub const HAIKU_4_5_NAME: &str = "Claude Haiku 4.5";

    /// Claude Opus 4.5 - Premium model combining maximum intelligence with practical performance
    /// Released: 2025-11-01
    pub const OPUS_4_5_ID: &str = "claude-opus-4-5-20251101";
    pub const OPUS_4_5_NAME: &str = "Claude Opus 4.5";

    /// Claude Opus 4.1 - Legacy premium model
    /// Released: 2025-08-05
    pub const OPUS_4_1_ID: &str = "claude-opus-4-1-20250805";
    pub const OPUS_4_1_NAME: &str = "Claude Opus 4.1";

    /// Claude Sonnet 4 - Legacy smart model
    /// Released: 2025-05-14
    pub const SONNET_4_ID: &str = "claude-sonnet-4-20250514";
    pub const SONNET_4_NAME: &str = "Claude Sonnet 4";

    // Backwards compatibility - default to Sonnet 4.5
    pub const SONNET_4_5: &str = SONNET_4_5_ID;
    pub const HAIKU_4_5: &str = HAIKU_4_5_ID;
    pub const OPUS_4_5: &str = OPUS_4_5_ID;
    pub const OPUS_4_1: &str = OPUS_4_1_ID;
    pub const SONNET_4: &str = SONNET_4_ID;
}

/// OpenAI model constants
pub mod openai {
    /// GPT-4o - Latest flagship model
    pub const GPT_4O_ID: &str = "gpt-4o";
    pub const GPT_4O_NAME: &str = "GPT-4o";

    /// GPT-4o Mini - Smaller, faster version of GPT-4o
    pub const GPT_4O_MINI_ID: &str = "gpt-4o-mini";
    pub const GPT_4O_MINI_NAME: &str = "GPT-4o Mini";

    /// GPT-4 Turbo - Enhanced GPT-4 model
    pub const GPT_4_TURBO_ID: &str = "gpt-4-turbo";
    pub const GPT_4_TURBO_NAME: &str = "GPT-4 Turbo";

    /// GPT-4 - Original GPT-4 model
    pub const GPT_4_ID: &str = "gpt-4";
    pub const GPT_4_NAME: &str = "GPT-4";

    /// GPT-3.5 Turbo - Fast and efficient model
    pub const GPT_3_5_TURBO_ID: &str = "gpt-3.5-turbo";
    pub const GPT_3_5_TURBO_NAME: &str = "GPT-3.5 Turbo";

    /// GPT-5 Codex - Advanced coding model (uses Responses API)
    pub const GPT_5_CODEX_ID: &str = "gpt-5-codex";
    pub const GPT_5_CODEX_NAME: &str = "GPT-5 Codex";

    /// GPT-5.1 - Next generation model (uses Responses API)
    pub const GPT_5_1_ID: &str = "gpt-5.1";
    pub const GPT_5_1_NAME: &str = "GPT-5.1";

    // Backwards compatibility
    pub const GPT_4O: &str = GPT_4O_ID;
    pub const GPT_4O_MINI: &str = GPT_4O_MINI_ID;
    pub const GPT_4_TURBO: &str = GPT_4_TURBO_ID;
    pub const GPT_4: &str = GPT_4_ID;
    pub const GPT_3_5_TURBO: &str = GPT_3_5_TURBO_ID;
    pub const GPT_5_CODEX: &str = GPT_5_CODEX_ID;
    pub const GPT_5_1: &str = GPT_5_1_ID;
}

/// xAI/Grok model constants
pub mod grok {
    /// Grok Beta - Latest Grok model
    pub const BETA_ID: &str = "grok-beta";
    pub const BETA_NAME: &str = "Grok Beta";

    /// Grok Vision Beta - Grok with vision capabilities
    pub const VISION_BETA_ID: &str = "grok-vision-beta";
    pub const VISION_BETA_NAME: &str = "Grok Vision Beta";

    /// Grok Code Fast 1 - Fast coding model
    pub const CODE_FAST_1_ID: &str = "grok-code-fast-1";
    pub const CODE_FAST_1_NAME: &str = "Grok Code Fast 1";

    // Backwards compatibility
    pub const BETA: &str = BETA_ID;
    pub const VISION_BETA: &str = VISION_BETA_ID;
    pub const CODE_FAST_1: &str = CODE_FAST_1_ID;
}

/// GLM model constants (via Cerebras/zAI)
pub mod glm {
    /// Llama 3.3 70B - Open source model via Cerebras
    pub const LLAMA_3_3_70B_ID: &str = "llama-3.3-70b";
    pub const LLAMA_3_3_70B_NAME: &str = "Llama 3.3 70B";

    /// zAI GLM 4.6 - GLM model via zAI provider
    pub const ZAI_GLM_4_6_ID: &str = "zai-glm-4.6";
    pub const ZAI_GLM_4_6_NAME: &str = "zAI GLM 4.6";

    // Backwards compatibility
    pub const LLAMA_3_3_70B: &str = LLAMA_3_3_70B_ID;
    pub const ZAI_GLM_4_6: &str = ZAI_GLM_4_6_ID;
}

/// Voyage AI embedding model constants
pub mod voyage {
    /// Voyage 4 Large - Highest accuracy embedding model
    /// Default dimension: 1024, supports 256/512/1024/2048
    pub const VOYAGE_4_LARGE_ID: &str = "voyage-4-large";
    pub const VOYAGE_4_LARGE_NAME: &str = "Voyage 4 Large";

    /// Voyage 4 - Balanced performance embedding model
    /// Default dimension: 1024, supports 256/512/1024/2048
    pub const VOYAGE_4_ID: &str = "voyage-4";
    pub const VOYAGE_4_NAME: &str = "Voyage 4";

    /// Voyage 4 Lite - Fast and cost-effective embedding model
    /// Default dimension: 1024, supports 256/512/1024/2048
    /// Max tokens: 1M per batch
    pub const VOYAGE_4_LITE_ID: &str = "voyage-4-lite";
    pub const VOYAGE_4_LITE_NAME: &str = "Voyage 4 Lite";

    /// Voyage 3 Large - Previous generation large model
    /// Default dimension: 1024, supports 256/512/1024/2048
    pub const VOYAGE_3_LARGE_ID: &str = "voyage-3-large";
    pub const VOYAGE_3_LARGE_NAME: &str = "Voyage 3 Large";

    /// Voyage 3.5 - Previous generation balanced model
    /// Default dimension: 1024, supports 256/512/1024/2048
    pub const VOYAGE_3_5_ID: &str = "voyage-3.5";
    pub const VOYAGE_3_5_NAME: &str = "Voyage 3.5";

    /// Voyage 3.5 Lite - Previous generation lite model
    /// Default dimension: 1024, supports 256/512/1024/2048
    pub const VOYAGE_3_5_LITE_ID: &str = "voyage-3.5-lite";
    pub const VOYAGE_3_5_LITE_NAME: &str = "Voyage 3.5 Lite";

    /// Voyage Code 3 - Specialized for code embeddings
    /// Default dimension: 1024, supports 256/512/1024/2048
    pub const VOYAGE_CODE_3_ID: &str = "voyage-code-3";
    pub const VOYAGE_CODE_3_NAME: &str = "Voyage Code 3";

    /// Voyage Finance 2 - Specialized for finance domain
    pub const VOYAGE_FINANCE_2_ID: &str = "voyage-finance-2";
    pub const VOYAGE_FINANCE_2_NAME: &str = "Voyage Finance 2";

    /// Voyage Law 2 - Specialized for legal domain
    pub const VOYAGE_LAW_2_ID: &str = "voyage-law-2";
    pub const VOYAGE_LAW_2_NAME: &str = "Voyage Law 2";

    // Backwards compatibility
    pub const VOYAGE_4_LARGE: &str = VOYAGE_4_LARGE_ID;
    pub const VOYAGE_4: &str = VOYAGE_4_ID;
    pub const VOYAGE_4_LITE: &str = VOYAGE_4_LITE_ID;
    pub const VOYAGE_3_LARGE: &str = VOYAGE_3_LARGE_ID;
    pub const VOYAGE_3_5: &str = VOYAGE_3_5_ID;
    pub const VOYAGE_3_5_LITE: &str = VOYAGE_3_5_LITE_ID;
    pub const VOYAGE_CODE_3: &str = VOYAGE_CODE_3_ID;
    pub const VOYAGE_FINANCE_2: &str = VOYAGE_FINANCE_2_ID;
    pub const VOYAGE_LAW_2: &str = VOYAGE_LAW_2_ID;
}

/// Google Gemini model constants
pub mod gemini {
    /// Gemini 3 Pro - Most intelligent model for complex reasoning
    /// Released: Preview, Context: 1M/64k, Thinking: low/high
    pub const GEMINI_3_PRO_ID: &str = "gemini-3-pro-preview";
    pub const GEMINI_3_PRO_NAME: &str = "Gemini 3 Pro";

    /// Gemini 3 Flash - Pro-level intelligence at Flash speed
    /// Released: Preview, Context: 1M/64k, Thinking: minimal/low/medium/high
    pub const GEMINI_3_FLASH_ID: &str = "gemini-3-flash-preview";
    pub const GEMINI_3_FLASH_NAME: &str = "Gemini 3 Flash";

    // Backwards compatibility
    pub const GEMINI_3_PRO: &str = GEMINI_3_PRO_ID;
    pub const GEMINI_3_FLASH: &str = GEMINI_3_FLASH_ID;
}

// Re-export for convenience
pub use claude::*;
