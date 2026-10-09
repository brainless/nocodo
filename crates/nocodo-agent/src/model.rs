use nocodo_types::project::{ModelConfig, Provider};
use rig_core::providers::anthropic::{Anthropic, Messages};

use crate::{Error, Result};

/// The environment variable that holds the Anthropic API key.
pub const API_KEY_VAR: &str = "ANTHROPIC_API_KEY";

/// The concrete completion model built from a [`ModelConfig`].
pub type AgentModel = rig_core::driver::Model<Messages>;

/// Builds the completion model for `config`, reading the API key from
/// `ANTHROPIC_API_KEY`. A missing or empty key is an error naming the variable.
pub fn build_model(config: &ModelConfig) -> Result<AgentModel> {
    match config.provider {
        Provider::Anthropic => {
            let key = std::env::var(API_KEY_VAR)
                .ok()
                .filter(|k| !k.trim().is_empty())
                .ok_or(Error::MissingApiKey(API_KEY_VAR))?;
            Ok(Anthropic::new(key.trim().to_string()).completion(config.model.clone()))
        }
    }
}
