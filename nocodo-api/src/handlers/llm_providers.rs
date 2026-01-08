use actix_web::{get, HttpResponse, Responder};
use nocodo_llm_sdk::model_metadata;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub model_id: String,
    pub name: String,
    pub context_length: u32,
    pub supports_streaming: bool,
    pub supports_tool_calling: bool,
    pub supports_vision: bool,
    pub supports_reasoning: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_cost_per_token: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_cost_per_token: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProvidersResponse {
    pub providers: HashMap<String, Vec<ModelInfo>>,
}

#[get("/providers")]
pub async fn list_providers() -> impl Responder {
    let models = model_metadata::get_all_models();

    let mut providers: HashMap<String, Vec<ModelInfo>> = HashMap::new();

    for model in models {
        let model_info = ModelInfo {
            model_id: model.model_id.to_string(),
            name: model.name.to_string(),
            context_length: model.context_length,
            supports_streaming: model.supports_streaming,
            supports_tool_calling: model.supports_tool_calling,
            supports_vision: model.supports_vision,
            supports_reasoning: model.supports_reasoning,
            input_cost_per_token: model.input_cost_per_token,
            output_cost_per_token: model.output_cost_per_token,
            default_temperature: model.default_temperature,
            default_max_tokens: model.default_max_tokens,
        };

        providers
            .entry(model.provider.to_string())
            .or_insert_with(Vec::new)
            .push(model_info);
    }

    HttpResponse::Ok().json(ProvidersResponse { providers })
}
