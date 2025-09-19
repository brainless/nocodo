pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod llm_agent;
pub mod llm_client;
pub mod models;
pub mod socket;
pub mod templates;
pub mod tools;
pub mod websocket;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod ts_bindings_tests {
    use super::models::*;
    use ts_rs::TS;

    #[test]
    fn export_ts_bindings() {
        Project::export().expect("Failed to export Project bindings");
        ProjectComponent::export().expect("Failed to export ProjectComponent bindings");
        ProjectDetailsResponse::export().expect("Failed to export ProjectDetailsResponse bindings");
        CreateProjectRequest::export().expect("Failed to export CreateProjectRequest bindings");
        ProjectResponse::export().expect("Failed to export ProjectResponse bindings");
        ProjectListResponse::export().expect("Failed to export ProjectListResponse bindings");
        ServerStatus::export().expect("Failed to export ServerStatus bindings");
        AiSession::export().expect("Failed to export AiSession bindings");
        CreateAiSessionRequest::export().expect("Failed to export CreateAiSessionRequest bindings");
        AiSessionResponse::export().expect("Failed to export AiSessionResponse bindings");
        AiSessionListResponse::export().expect("Failed to export AiSessionListResponse bindings");
        AiSessionOutput::export().expect("Failed to export AiSessionOutput bindings");
        AiSessionResult::export().expect("Failed to export AiSessionResult bindings");
        AiSessionOutputListResponse::export()
            .expect("Failed to export AiSessionOutputListResponse bindings");
        RecordAiOutputRequest::export().expect("Failed to export RecordAiOutputRequest bindings");
        AddExistingProjectRequest::export()
            .expect("Failed to export AddExistingProjectRequest bindings");
        FileInfo::export().expect("Failed to export FileInfo bindings");
        FileListRequest::export().expect("Failed to export FileListRequest bindings");
        FileListResponse::export().expect("Failed to export FileListResponse bindings");
        FileCreateRequest::export().expect("Failed to export FileCreateRequest bindings");
        FileUpdateRequest::export().expect("Failed to export FileUpdateRequest bindings");
        FileContentResponse::export().expect("Failed to export FileContentResponse bindings");
        FileResponse::export().expect("Failed to export FileResponse bindings");
        Work::export().expect("Failed to export Work bindings");
        WorkMessage::export().expect("Failed to export WorkMessage bindings");
        WorkWithHistory::export().expect("Failed to export WorkWithHistory bindings");
        MessageContentType::export().expect("Failed to export MessageContentType bindings");
        MessageAuthorType::export().expect("Failed to export MessageAuthorType bindings");
        CreateWorkRequest::export().expect("Failed to export CreateWorkRequest bindings");
        WorkResponse::export().expect("Failed to export WorkResponse bindings");
        WorkListResponse::export().expect("Failed to export WorkListResponse bindings");
        WorkMessageResponse::export().expect("Failed to export WorkMessageResponse bindings");
        WorkMessageListResponse::export()
            .expect("Failed to export WorkMessageListResponse bindings");
        AddMessageRequest::export().expect("Failed to export AddMessageRequest bindings");
    }
}
