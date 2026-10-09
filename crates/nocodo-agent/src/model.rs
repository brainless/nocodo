use nocodo_types::project::ModelConfig;
use rig_core::{
    DynModel,
    operation::Completion,
    providers::{
        anthropic::wire::AnthropicConfig,
        openai::OpenAIConfig,
        registry::{ProviderConfig, ProviderId, ProviderRef},
    },
};

use crate::{Error, Result};

/// A built completion model and whether strict tool schemas are on for it.
pub struct BuiltModel {
    pub model: DynModel<Completion>,
    /// True when the provider constrains `submit` arguments to the schema.
    pub strict: bool,
}

/// Builds the completion model for `config`.
///
/// `config.provider` is a registry selection (`anthropic`, `openai`, `gemini`,
/// `llamacpp`, `neuralwatt`, ...). The API key comes from the variable the registry names
/// for that provider; providers that need no credential (local servers) skip
/// the check.
///
/// Strict tool schemas are opt-in (`ModelConfig::strict`, off by default) and
/// only take effect for providers that support them. They are a wire option that a registry-built model cannot
/// carry in Rig 0.43 (`ProviderConfig` has no field for it), so Anthropic-format
/// providers and OpenAI itself are built through their own clients with
/// `with_strict_tools()` on. Every other provider goes through the registry and
/// relies on the retry loop in `run_extractor`.
pub fn build_model(config: &ModelConfig) -> Result<BuiltModel> {
    if config.provider == NEURALWATT {
        let key = std::env::var(NEURALWATT_KEY_ENV).ok();
        return build_model_with_key(config, key.as_deref());
    }
    let key = ProviderId::resolve(&config.provider)
        .ok()
        .and_then(|id| std::env::var(id.api_key_env()).ok());
    build_model_with_key(config, key.as_deref())
}

/// Provider selection for Neuralwatt, which Rig 0.43's registry does not know.
const NEURALWATT: &str = "neuralwatt";
const NEURALWATT_KEY_ENV: &str = "NEURALWATT_API_KEY";
const NEURALWATT_BASE_URL: &str = "https://api.neuralwatt.com/v1";

/// Neuralwatt speaks OpenAI chat completions, so it is built from the OpenAI
/// config with its base URL. Strict tool schemas stay off; the retry loop in
/// `run_extractor` covers invalid output.
fn build_neuralwatt(config: &ModelConfig, key: Option<&str>) -> Result<BuiltModel> {
    let Some(key) = key.filter(|k| !k.trim().is_empty()) else {
        return Err(Error::MissingApiKey {
            provider: "Neuralwatt".to_string(),
            var: NEURALWATT_KEY_ENV,
        });
    };
    let model = OpenAIConfig::new(key)
        .with_base_url(NEURALWATT_BASE_URL)
        .client()
        .chat(config.model.clone());
    Ok(BuiltModel {
        model: model.into(),
        strict: false,
    })
}

/// [`build_model`] with the API key given instead of read from the environment.
fn build_model_with_key(config: &ModelConfig, key: Option<&str>) -> Result<BuiltModel> {
    if config.provider == NEURALWATT {
        return build_neuralwatt(config, key);
    }
    let id = ProviderId::resolve(&config.provider).map_err(|e| Error::UnknownProvider {
        provider: config.provider.clone(),
        reason: e.to_string(),
        valid: valid_selections(),
    })?;
    if id.requires_credential() && !key.is_some_and(|k| !k.trim().is_empty()) {
        return Err(Error::MissingApiKey {
            provider: id.to_string(),
            var: id.api_key_env(),
        });
    }
    let build_error = |message: String| Error::ModelBuild {
        provider: config.provider.clone(),
        message,
    };
    let key = key.unwrap_or_default();
    match id.config(key) {
        ProviderConfig::Anthropic(cfg) => {
            // Re-read the dialect's optional base URL variable as well as the key.
            let cfg = AnthropicConfig::from_env_with(&cfg.dialect).unwrap_or(cfg);
            let strict = config.strict && cfg.dialect.quirks.strict_tool_schemas;
            let mut model = cfg.client().completion(config.model.clone());
            if strict {
                model.wire = model.wire.with_strict_tools();
            }
            Ok(BuiltModel {
                model: model.into(),
                strict,
            })
        }
        ProviderConfig::OpenAi(cfg) if id.vendor() == "openai" => {
            let mut model = cfg.client().completion(config.model.clone());
            if config.strict {
                model.wire = model.wire.with_strict_tools();
            }
            Ok(BuiltModel {
                model: model.into(),
                strict: config.strict,
            })
        }
        _ => {
            let model = ProviderRef::registered(id, config.model.clone())
                .map_err(|e| build_error(e.to_string()))?
                .completion_model()
                .map_err(|e| build_error(e.to_string()))?;
            Ok(BuiltModel {
                model,
                strict: false,
            })
        }
    }
}

/// Every provider selection the registry accepts, comma separated.
fn valid_selections() -> String {
    ProviderId::all()
        .map(|id| id.to_string())
        .chain(std::iter::once(NEURALWATT.to_string()))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(provider: &str) -> ModelConfig {
        ModelConfig {
            provider: provider.to_string(),
            model: "some-model".to_string(),
            strict: false,
        }
    }

    fn strict_config(provider: &str) -> ModelConfig {
        ModelConfig {
            strict: true,
            ..config(provider)
        }
    }

    #[test]
    fn strict_tools_are_off_by_default() {
        for provider in ["anthropic", "openai"] {
            let built = build_model_with_key(&config(provider), Some("test-key")).unwrap();
            assert!(!built.strict, "{provider} should default to non-strict");
            assert_eq!(built.model.id(), Some("some-model"));
        }
    }

    #[test]
    fn anthropic_and_openai_use_strict_tools_when_requested() {
        for provider in ["anthropic", "openai"] {
            let built = build_model_with_key(&strict_config(provider), Some("test-key")).unwrap();
            assert!(built.strict, "{provider} should use strict tools");
        }
    }

    #[test]
    fn strict_requested_on_unsupported_providers_is_false() {
        let built = build_model_with_key(&strict_config("neuralwatt"), Some("k")).unwrap();
        assert!(!built.strict);
        let built = build_model_with_key(&strict_config("llamacpp"), None).unwrap();
        assert!(!built.strict);
    }

    #[test]
    fn missing_key_is_reported_before_building() {
        let Err(err) = build_model_with_key(&config("openai"), None) else {
            panic!("should fail");
        };
        assert!(err.to_string().contains("OPENAI_API_KEY"), "{err}");
    }

    #[test]
    fn neuralwatt_builds_with_key_and_without_strict() {
        let built = build_model_with_key(&config("neuralwatt"), Some("test-key")).unwrap();
        assert!(!built.strict);
        assert_eq!(built.model.id(), Some("some-model"));
    }

    #[test]
    fn neuralwatt_missing_key_names_the_variable() {
        for key in [None, Some("  ")] {
            let Err(err) = build_model_with_key(&config("neuralwatt"), key) else {
                panic!("should fail");
            };
            assert!(err.to_string().contains("NEURALWATT_API_KEY"), "{err}");
        }
    }

    #[test]
    fn unknown_provider_lists_neuralwatt() {
        let Err(err) = build_model_with_key(&config("nope"), Some("k")) else {
            panic!("should fail");
        };
        assert!(err.to_string().contains("neuralwatt"), "{err}");
    }

    #[test]
    fn local_servers_go_through_the_registry_without_strict() {
        let built = build_model_with_key(&config("llamacpp"), None).unwrap();
        assert!(!built.strict);
    }
}
