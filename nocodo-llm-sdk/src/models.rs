//! Model constants for supported LLM providers
//!
//! This module contains official model IDs and metadata for all supported providers.
//! Model IDs are sourced from official provider documentation.

/// Claude model constants
pub mod claude {
    /// Claude Sonnet 4.5 - Smart model for complex agents and coding
    /// Released: 2025-09-29
    pub const SONNET_4_5: &str = "claude-sonnet-4-5-20250929";

    /// Claude Haiku 4.5 - Fastest model with near-frontier intelligence
    /// Released: 2025-10-01
    pub const HAIKU_4_5: &str = "claude-haiku-4-5-20251001";

    /// Claude Opus 4.5 - Premium model combining maximum intelligence with practical performance
    /// Released: 2025-11-01
    pub const OPUS_4_5: &str = "claude-opus-4-5-20251101";

    /// Claude Opus 4.1 - Legacy premium model
    /// Released: 2025-08-05
    pub const OPUS_4_1: &str = "claude-opus-4-1-20250805";

    /// Claude Sonnet 4 - Legacy smart model
    /// Released: 2025-05-14
    pub const SONNET_4: &str = "claude-sonnet-4-20250514";
}

/// OpenAI model constants
pub mod openai {
    /// GPT-4o - Latest flagship model
    pub const GPT_4O: &str = "gpt-4o";

    /// GPT-4o Mini - Smaller, faster version of GPT-4o
    pub const GPT_4O_MINI: &str = "gpt-4o-mini";

    /// GPT-4 Turbo - Enhanced GPT-4 model
    pub const GPT_4_TURBO: &str = "gpt-4-turbo";

    /// GPT-4 - Original GPT-4 model
    pub const GPT_4: &str = "gpt-4";

    /// GPT-3.5 Turbo - Fast and efficient model
    pub const GPT_3_5_TURBO: &str = "gpt-3.5-turbo";

    /// GPT-5 Codex - Advanced coding model (uses Responses API)
    pub const GPT_5_CODEX: &str = "gpt-5-codex";

    /// GPT-5.1 - Next generation model (uses Responses API)
    pub const GPT_5_1: &str = "gpt-5.1";
}

/// xAI/Grok model constants
pub mod grok {
    /// Grok Beta - Latest Grok model
    pub const BETA: &str = "grok-beta";

    /// Grok Vision Beta - Grok with vision capabilities
    pub const VISION_BETA: &str = "grok-vision-beta";

    /// Grok Code Fast 1 - Fast coding model
    pub const CODE_FAST_1: &str = "grok-code-fast-1";
}

/// GLM model constants (via Cerebras/zAI)
pub mod glm {
    /// Llama 3.3 70B - Open source model via Cerebras
    pub const LLAMA_3_3_70B: &str = "llama-3.3-70b";

    /// zAI GLM 4.6 - GLM model via zAI provider
    pub const ZAI_GLM_4_6: &str = "zai-glm-4.6";
}

// Re-export for convenience
pub use claude::*;
