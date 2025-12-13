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

// Re-export for convenience
pub use claude::*;
