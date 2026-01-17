use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::{
    error::LlmError,
    voyage::types::{VoyageEmbeddingRequest, VoyageEmbeddingResponse, VoyageErrorResponse},
};

/// Voyage AI client for text embeddings
pub struct VoyageClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl VoyageClient {
    /// Create a new Voyage AI client with the given API key
    pub fn new(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::authentication("API key cannot be empty"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key,
            base_url: "https://api.voyageai.com".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create embeddings for the given input
    pub async fn create_embedding(
        &self,
        request: VoyageEmbeddingRequest,
    ) -> Result<VoyageEmbeddingResponse, LlmError> {
        let url = format!("{}/v1/embeddings", self.base_url);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .map_err(|_| LlmError::authentication("Invalid API key format"))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Network { source: e })?;

        let status = response.status();

        if status.is_success() {
            let voyage_response: VoyageEmbeddingResponse = response
                .json()
                .await
                .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;
            Ok(voyage_response)
        } else {
            // Extract retry-after header before consuming the response
            let retry_after = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
            } else {
                None
            };

            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as Voyage error response
            if let Ok(error_response) = serde_json::from_str::<VoyageErrorResponse>(&error_text) {
                match status {
                    reqwest::StatusCode::BAD_REQUEST => {
                        Err(LlmError::invalid_request(error_response.detail))
                    }
                    reqwest::StatusCode::UNAUTHORIZED => {
                        Err(LlmError::authentication(error_response.detail))
                    }
                    reqwest::StatusCode::FORBIDDEN => {
                        Err(LlmError::authentication(error_response.detail))
                    }
                    reqwest::StatusCode::NOT_FOUND => {
                        Err(LlmError::api_error(404, error_response.detail))
                    }
                    reqwest::StatusCode::PAYLOAD_TOO_LARGE => {
                        Err(LlmError::invalid_request("Request too large"))
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        Err(LlmError::rate_limit(error_response.detail, retry_after))
                    }
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                        Err(LlmError::api_error(500, error_response.detail))
                    }
                    _ => Err(LlmError::api_error(
                        status.as_u16(),
                        error_response.detail,
                    )),
                }
            } else {
                // Fallback for non-standard error responses
                match status {
                    reqwest::StatusCode::BAD_REQUEST => Err(LlmError::invalid_request(error_text)),
                    reqwest::StatusCode::UNAUTHORIZED => Err(LlmError::authentication(error_text)),
                    reqwest::StatusCode::FORBIDDEN => Err(LlmError::authentication(error_text)),
                    reqwest::StatusCode::NOT_FOUND => Err(LlmError::api_error(404, error_text)),
                    reqwest::StatusCode::PAYLOAD_TOO_LARGE => {
                        Err(LlmError::invalid_request("Request too large"))
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        Err(LlmError::rate_limit(error_text, retry_after))
                    }
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                        Err(LlmError::api_error(500, error_text))
                    }
                    _ => Err(LlmError::api_error(status.as_u16(), error_text)),
                }
            }
        }
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        crate::providers::VOYAGE
    }
}
