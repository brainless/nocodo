use manager_models::{Project, ProjectListResponse};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiClient {
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    fn client(&self) -> reqwest::Client {
        reqwest::Client::new()
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, ApiError> {
        let url = format!("{}/api/projects", self.base_url);
        let response = self.client()
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let project_response: ProjectListResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(project_response.projects)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("HTTP error: {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("Parse failed: {0}")]
    ParseFailed(String),
}