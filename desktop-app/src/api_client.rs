use manager_models::{
    CreateWorkRequest, Project, ProjectListResponse, SettingsResponse, Work, WorkListResponse, WorkResponse,
};

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
        let response = self
            .client()
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

    pub async fn list_works(&self) -> Result<Vec<Work>, ApiError> {
        let url = format!("{}/api/work", self.base_url);
        let response = self
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let work_response: WorkListResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(work_response.works)
    }

    pub async fn get_work_messages(
        &self,
        work_id: i64,
    ) -> Result<Vec<manager_models::WorkMessage>, ApiError> {
        let url = format!("{}/api/work/{}/messages", self.base_url, work_id);
        let response = self
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let messages_response: manager_models::WorkMessageListResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(messages_response.messages)
    }

    pub async fn get_ai_session_outputs(
        &self,
        work_id: i64,
    ) -> Result<Vec<manager_models::AiSessionOutput>, ApiError> {
        let url = format!("{}/api/work/{}/outputs", self.base_url, work_id);
        let response = self
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let outputs_response: manager_models::AiSessionOutputListResponse =
            response
                .json()
                .await
                .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(outputs_response.outputs)
    }

    pub async fn create_work(&self, request: CreateWorkRequest) -> Result<Work, ApiError> {
        let url = format!("{}/api/work", self.base_url);
        let response = self
            .client()
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let work_response: WorkResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(work_response.work)
    }

    pub async fn get_settings(&self) -> Result<SettingsResponse, ApiError> {
        let url = format!("{}/settings", self.base_url);
        let response = self
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let settings_response: SettingsResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(settings_response)
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
