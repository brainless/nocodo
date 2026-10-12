pub mod code_extractor;
// COMMENTED OUT (Praxis v1 Phase 1): epic/task comments disabled with epics/tasks.
// pub mod comments;
// COMMENTED OUT (Praxis v1 Phase 1): context gathering was specialist-agent support infra.
// pub mod context;
// COMMENTED OUT (Praxis v1 Phase 1): db_engineer was a task-execution specialist agent.
// pub mod db_engineer;
// COMMENTED OUT (Praxis v1 Phase 1): dispatcher drove the old task state machine.
// pub mod dispatcher;
// COMMENTED OUT (Praxis v1 Phase 1): project_notes replaced by direct praxis spec files.
// pub mod project_notes;
pub mod rust_engineer;
pub mod stack_reviewer;
pub mod state;
// COMMENTED OUT (Praxis v1 Phase 1): ui_designer was a task-execution specialist agent.
// pub mod ui_designer;
pub mod user_chat;

pub use state::AgentState;
