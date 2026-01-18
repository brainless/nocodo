use super::types::*;
use crate::error::LlmError;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

/// Google Gemini API client
pub struct GeminiClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl GeminiClient {
    pub fn new(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::authentication("API key cannot be empty"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key,
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            http_client,
        })
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub async fn generate_content(
        &self,
        model: impl Into<String>,
        request: GeminiGenerateContentRequest,
    ) -> Result<GeminiGenerateContentResponse, LlmError> {
        let model = model.into();
        let url = format!("{}/v1beta/models/{}:generateContent", self.base_url, model);

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-goog-api-key",
            HeaderValue::from_str(&self.api_key)
                .map_err(|e| LlmError::authentication(format!("Invalid API key format: {}", e)))?,
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

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());

            if let Ok(error_response) = serde_json::from_str::<GeminiErrorResponse>(&error_body) {
                return Err(Self::map_error(
                    error_response.error.code,
                    error_response.error.message,
                ));
            }

            return Err(LlmError::api_error(status.as_u16(), error_body));
        }

        let generate_response = response
            .json::<GeminiGenerateContentResponse>()
            .await
            .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;

        Ok(generate_response)
    }

    fn map_error(status: u16, message: String) -> LlmError {
        match status {
            400 => LlmError::invalid_request(message),
            401 | 403 => LlmError::Authentication { message },
            404 => LlmError::api_error(status, message),
            429 => LlmError::rate_limit(message, None),
            500 | 503 => LlmError::api_error(status, message),
            _ => LlmError::api_error(status, message),
        }
    }

    pub fn provider_name(&self) -> &str {
        "Google"
    }

    pub fn message_builder(&self) -> super::builder::MessageBuilder<'_> {
        super::builder::MessageBuilder::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GeminiClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = GeminiClient::new("");
        assert!(client.is_err());
    }
}
