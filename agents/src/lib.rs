// COMMENTED OUT (Praxis v1 Phase 1): backend_engineer was a task-execution specialist agent.
// pub mod backend_engineer;
pub mod checker;
pub mod code_extractor;
pub mod code_writer;
pub mod codebase_index;
pub mod config;
// COMMENTED OUT (Praxis v1 Phase 1): db_engineer was a task-execution specialist agent.
// pub mod db_engineer;
pub mod error;
// COMMENTED OUT (Praxis v1 Phase 1): frontend_engineer was a task-execution specialist agent.
// pub mod frontend_engineer;
pub mod nocodo_description;
pub mod praxis_doc;
pub mod product_owner;
// COMMENTED OUT (Praxis v1 Phase 1): PM replaced by direct PO -> RustEngineer loop.
// pub mod project_manager;
pub mod rust_engineer;
pub mod spec_fact;
pub mod spec_gap;
pub mod stack_reviewer;
pub mod static_checker;
pub mod storage;
pub mod task_policy;
// COMMENTED OUT (Praxis v1 Phase 1): ui_designer was a task-execution specialist agent.
// pub mod ui_designer;
pub mod user_input_tool;
pub mod utils;

pub use nocodo_description::NOCODO_DESCRIPTION;

// COMMENTED OUT (Praxis v1 Phase 1): backend_engineer module disabled above.
// pub use backend_engineer::{BackendEngineerAgent, BackendEngineerResponse};
pub use checker::{run_cargo_check, CargoCheckOutcome};
pub use config::AgentConfig;
// COMMENTED OUT (Praxis v1 Phase 1): db_engineer module disabled above.
// pub use db_engineer::{AgentResponse, DbEngineerAgent, StopAgentParams};
pub use error::AgentError;
// COMMENTED OUT (Praxis v1 Phase 1): frontend_engineer module disabled above.
// pub use frontend_engineer::{FrontendEngineerAgent, FrontendEngineerResponse};
pub use product_owner::{
    CompleteGapClarificationParams, CompletePersonaInterviewParams, CompleteRequirementsParams,
    FinalizeSessionParams, PoMode, PoSessionResult, ProductOwnerAgent, RecordProjectNoteParams,
    SetProjectNameParams, TriggerCodegenParams,
};
// COMMENTED OUT (Praxis v1 Phase 1): project_manager module disabled above; PM replaced by loop.
// pub use project_manager::{
//     FinalizeSessionParams, FinalizeTaskDef, PmResponse, PmUserSessionResult, ProjectManagerAgent,
// };
pub use rust_engineer::{
    DieselModelFnOutput, DieselModelStructOutput, DieselSchemaOutput, PraxisAuthOutput,
    RustEngineerAgent, RustEngineerResult, StateTransitionOutput, TransitionMetadata,
};
pub use spec_fact::{record_spec_fact, RecordSpecFactParams, RecordSpecFactResult};
pub use spec_gap::{
    find_gaps_from_persona_notes, find_pending_gaps_in_code, gaps_to_questions, GapQuestion,
    SpecGap,
};
pub use stack_reviewer::{StackReviewResult, StackReviewerAgent};
pub use static_checker::{
    check_guard_coverage, check_referential_integrity, check_spec_completeness,
    check_unresolved, extract_id_refs, load_transition_guard_code, map_gaps_to_questions,
    run_static_checks, Gap, IdKind, IdRef, IdRefs, PoQuestion, SpecIndex, TransitionSpec,
};
pub use storage::spec_schemas::{PersonaNote, PraxisWriterTaskSpec, SpecNoteContent};
pub use storage::sqlite::{
    SqliteAgentStorage, SqliteCommentStorage, SqliteContextStorage, SqliteProjectNoteStorage,
    SqliteSchemaStorage, SqliteStackNoteStorage, SqliteTaskStorage, SqliteUiFormStorage,
    SqliteUserChatStorage, SqliteUserStorage,
};
pub use storage::{
    AgentStorage, AgentType, ChatMessage, CommentStorage, ContextStorage, Epic, EpicStatus,
    MessageContent, ProjectNoteRow, ProjectNoteStorage, ProjectNoteTopic, QuestionKind,
    SchemaStorage, Session, StackNoteRow, StackNoteStorage, StackTag, StructuredQuestion,
    StructuredResponse, Task, TaskStatus, TaskStorage, UiFormStorage, UserChatMessageRow,
    UserChatSessionRow, UserChatStorage, UserStorage,
};
// COMMENTED OUT (Praxis v1 Phase 1): ui_designer module disabled above.
// pub use ui_designer::{
//     agent::{UiDesignerAgent, UiDesignerResponse},
//     FormField, FormFieldType, FormLayout, FormRow,
// };
pub use user_input_tool::{InputType, RequestUserInputParams};

// ---------------------------------------------------------------------------
// Factory helpers
// ---------------------------------------------------------------------------

use std::sync::Arc;

pub(crate) fn make_llm_client(
    config: &AgentConfig,
) -> Result<Arc<dyn llm_sdk::client::LlmClient>, AgentError> {
    use llm_sdk::{claude::ClaudeClient, groq::GroqClient, openai::OpenAIClient};
    let client: Arc<dyn llm_sdk::client::LlmClient> = match config.provider.as_str() {
        config::PROVIDER_ANTHROPIC => Arc::new(
            ClaudeClient::new(config.api_key.clone())
                .map_err(|e| AgentError::Config(e.to_string()))?,
        ),
        config::PROVIDER_GROQ => Arc::new(
            GroqClient::new(config.api_key.clone())
                .map_err(|e| AgentError::Config(e.to_string()))?,
        ),
        _ => Arc::new(
            OpenAIClient::new(config.api_key.clone())
                .map_err(|e| AgentError::Config(e.to_string()))?,
        ),
    };
    Ok(client)
}

/* COMMENTED OUT (Praxis v1 Phase 1): factory functions for disabled specialist agents
 * (db_engineer, project_manager, ui_designer, backend_engineer, frontend_engineer).
 * These construct agents whose modules are commented out above; PM's role is
 * replaced by the direct PO -> RustEngineer loop, and specialists had no more
 * tasks to execute once the dispatcher is disabled.
 *
pub fn build_db_engineer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
) -> Result<DbEngineerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let schema_storage: Arc<dyn SchemaStorage> = Arc::new(SqliteSchemaStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(DbEngineerAgent::new(
        client,
        storage,
        schema_storage,
        task_storage,
        &config.model,
        project_id,
    ))
}

pub fn build_project_manager(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
) -> Result<ProjectManagerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(ProjectManagerAgent::new(
        client,
        storage,
        task_storage,
        &config.model,
        project_id,
    ))
}

pub fn build_ui_designer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
) -> Result<UiDesignerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let form_storage: Arc<dyn UiFormStorage> = Arc::new(SqliteUiFormStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(UiDesignerAgent::new(
        client,
        storage,
        form_storage,
        task_storage,
        &config.model,
        project_id,
    ))
}

pub fn build_project_manager_with_task_storage(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
    task_storage: Arc<dyn TaskStorage>,
) -> Result<ProjectManagerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);

    Ok(ProjectManagerAgent::new(
        client,
        storage,
        task_storage,
        &config.model,
        project_id,
    ))
}

pub fn build_backend_engineer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
    project_path: &str,
) -> Result<BackendEngineerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let context_storage: Arc<dyn ContextStorage> = Arc::new(SqliteContextStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(BackendEngineerAgent::new(
        client,
        storage,
        context_storage,
        task_storage,
        &config.model,
        project_id,
        project_path,
    ))
}

pub fn build_frontend_engineer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
    project_path: &str,
) -> Result<FrontendEngineerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let context_storage: Arc<dyn ContextStorage> = Arc::new(SqliteContextStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(FrontendEngineerAgent::new(
        client,
        storage,
        context_storage,
        task_storage,
        &config.model,
        project_id,
        project_path,
    ))
}
 */

pub fn build_rust_engineer(project_path: &str) -> Result<RustEngineerAgent, AgentError> {
    let model = std::env::var("RUST_ENGINEER_MODEL")
        .unwrap_or_else(|_| "unsloth/Qwen3.5-0.8B-GGUF:UD-Q4_K_XL".to_string());
    let base_url = std::env::var("LLAMA_CPP_BASE_URL").ok();
    RustEngineerAgent::new(model, base_url, project_path)
}

pub fn build_stack_reviewer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
    project_path: &str,
) -> Result<StackReviewerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let stack_note_storage: Arc<dyn StackNoteStorage> =
        Arc::new(SqliteStackNoteStorage::open(db_path)?);
    Ok(StackReviewerAgent::new(
        client,
        stack_note_storage,
        &config.model,
        project_id,
        project_path,
    ))
}
