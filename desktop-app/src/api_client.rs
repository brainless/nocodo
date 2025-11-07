use manager_models::{
    CreateWorkRequest, FileContentResponse, FileInfo, Project, ProjectDetailsResponse,
    ProjectListResponse, SearchQuery, ServerStatus, SettingsResponse, SupportedModelsResponse,
    Team, TeamListResponse, UpdateApiKeysRequest, UpdateUserRequest, User, UserDetailResponse,
    UserListResponse, Work, WorkListResponse, WorkResponse,
};
use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiClient {
    base_url: String,
    #[serde(skip)]
    jwt_token: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            jwt_token: None,
        }
    }

    pub fn set_jwt_token(&mut self, token: Option<String>) {
        self.jwt_token = token;
    }

    pub fn get_jwt_token(&self) -> Option<String> {
        self.jwt_token.clone()
    }

    fn client(&self) -> reqwest::Client {
        reqwest::Client::new()
    }

    fn add_auth_header(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(token) = &self.jwt_token {
            request.header("Authorization", format!("Bearer {}", token))
        } else {
            request
        }
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, ApiError> {
        let url = format!("{}/api/projects", self.base_url);
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
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

    pub async fn get_project_details(
        &self,
        project_id: i64,
    ) -> Result<ProjectDetailsResponse, ApiError> {
        let url = format!("{}/api/projects/{}/details", self.base_url, project_id);
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let project_details_response: ProjectDetailsResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(project_details_response)
    }

    pub async fn list_works(&self) -> Result<Vec<Work>, ApiError> {
        let url = format!("{}/api/work", self.base_url);
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
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
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
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
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
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

    pub async fn get_ai_tool_calls(
        &self,
        work_id: i64,
    ) -> Result<Vec<manager_models::LlmAgentToolCall>, ApiError> {
        let url = format!("{}/api/work/{}/tool-calls", self.base_url, work_id);
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let tool_calls_response: manager_models::LlmAgentToolCallListResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(tool_calls_response.tool_calls)
    }

    pub async fn create_work(&self, request: CreateWorkRequest) -> Result<Work, ApiError> {
        let url = format!("{}/api/work", self.base_url);
        let req = self.client().post(&url).json(&request);
        let req = self.add_auth_header(req);
        let response = req
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

    pub async fn add_message_to_work(
        &self,
        work_id: i64,
        request: manager_models::AddMessageRequest,
    ) -> Result<manager_models::WorkMessage, ApiError> {
        let url = format!("{}/api/work/{}/messages", self.base_url, work_id);
        let req = self.client().post(&url).json(&request);
        let req = self.add_auth_header(req);
        let response = req
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let message_response: manager_models::WorkMessageResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(message_response.message)
    }

    pub async fn get_supported_models(
        &self,
    ) -> Result<Vec<manager_models::SupportedModel>, ApiError> {
        let url = format!("{}/api/models", self.base_url);
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let models_response: SupportedModelsResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(models_response.models)
    }

    pub async fn health_check(&self) -> Result<ServerStatus, ApiError> {
        let url = format!("{}/api/health", self.base_url);
        let response = self
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let status: ServerStatus = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(status)
    }

    pub async fn get_settings(&self) -> Result<SettingsResponse, ApiError> {
        let url = format!("{}/api/settings", self.base_url);
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
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

    pub async fn set_projects_default_path(&self, path: String) -> Result<Value, ApiError> {
        let url = format!("{}/api/settings/projects-path", self.base_url);
        let payload = serde_json::json!({
            "path": path
        });

        let req = self.client().post(&url).json(&payload);
        let req = self.add_auth_header(req);
        let response = req
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(result)
    }

    pub async fn scan_projects(&self) -> Result<Value, ApiError> {
        let url = format!("{}/api/projects/scan", self.base_url);

        let req = self.client().post(&url);
        let req = self.add_auth_header(req);
        let response = req
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(result)
    }

    pub async fn update_api_keys(&self, request: UpdateApiKeysRequest) -> Result<Value, ApiError> {
        let url = format!("{}/api/settings/api-keys", self.base_url);

        let req = self.client().post(&url).json(&request);
        let req = self.add_auth_header(req);
        let response = req
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(result)
    }

    pub async fn list_files(
        &self,
        project_id: i64,
        path: Option<&str>,
    ) -> Result<Vec<FileInfo>, ApiError> {
        let mut url = format!("{}/api/files?project_id={}", self.base_url, project_id);
        if let Some(path) = path {
            url.push_str(&format!("&path={}", urlencoding::encode(path)));
        }

        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let file_list_response: manager_models::FileListResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(file_list_response.files)
    }

    pub async fn get_file_content(
        &self,
        project_id: i64,
        path: &str,
    ) -> Result<FileContentResponse, ApiError> {
        let url = format!(
            "{}/api/files/{}?project_id={}",
            self.base_url,
            urlencoding::encode(path),
            project_id
        );

        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let file_content_response: FileContentResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(file_content_response)
    }

    /// Register a new user
    pub async fn register(
        &self,
        username: &str,
        password: &str,
        email: Option<&str>,
        ssh_public_key: &str,
        ssh_fingerprint: &str,
    ) -> Result<manager_models::UserResponse, ApiError> {
        let url = format!("{}/api/auth/register", self.base_url);
        let payload = manager_models::CreateUserRequest {
            username: username.to_string(),
            email: email.map(|s| s.to_string()),
            password: password.to_string(),
            ssh_public_key: ssh_public_key.to_string(),
            ssh_fingerprint: ssh_fingerprint.to_string(),
        };

        let response = self
            .client()
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let user_response: manager_models::UserResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(user_response)
    }

    /// Login with username, password, and SSH fingerprint
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        ssh_fingerprint: &str,
    ) -> Result<manager_models::LoginResponse, ApiError> {
        let url = format!("{}/api/auth/login", self.base_url);
        let payload = manager_models::LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
            ssh_fingerprint: ssh_fingerprint.to_string(),
        };

        let response = self
            .client()
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let login_response: manager_models::LoginResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(login_response)
    }

    pub async fn list_users(&self) -> Result<Vec<User>, ApiError> {
        let url = format!("{}/api/users", self.base_url);
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let user_response: UserListResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(user_response.users)
    }

    pub async fn get_user(&self, user_id: i64) -> Result<UserDetailResponse, ApiError> {
        let url = format!("{}/api/users/{}", self.base_url, user_id);
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let user_detail: UserDetailResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(user_detail)
    }

    pub async fn update_user(&self, user_id: i64, request: UpdateUserRequest) -> Result<(), ApiError> {
        let url = format!("{}/api/users/{}", self.base_url, user_id);
        let request_builder = self.client().patch(&url);
        let request_builder = self.add_auth_header(request_builder);
        let response = request_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        Ok(())
    }

    pub async fn search_users(&self, query: &str) -> Result<Vec<User>, ApiError> {
        let url = format!("{}/api/users/search?q={}", self.base_url, urlencoding::encode(query));
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let user_response: UserListResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(user_response.users)
    }

    pub async fn list_teams(&self) -> Result<Vec<Team>, ApiError> {
        let url = format!("{}/api/teams", self.base_url);
        let request = self.client().get(&url);
        let request = self.add_auth_header(request);
        let response = request
            .send()
            .await
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status()));
        }

        let team_response: TeamListResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ParseFailed(e.to_string()))?;

        Ok(team_response.teams)
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

impl ApiError {
    /// Check if this error is a 401 Unauthorized error (needs authentication)
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, ApiError::HttpStatus(status) if status == &reqwest::StatusCode::UNAUTHORIZED)
    }

    /// Check if this error is a 403 Forbidden error (permission denied)
    pub fn is_forbidden(&self) -> bool {
        matches!(self, ApiError::HttpStatus(status) if status == &reqwest::StatusCode::FORBIDDEN)
    }

    /// Check if this is an authentication-related error (401 or 403)
    pub fn is_auth_error(&self) -> bool {
        self.is_unauthorized() || self.is_forbidden()
    }
}
