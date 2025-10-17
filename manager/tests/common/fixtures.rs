use std::sync::atomic::{AtomicU64, Ordering};

use nocodo_manager::models::{
    AiSession, AiSessionOutput, AiSessionResult, LlmAgentMessage, LlmAgentSession,
    LlmAgentToolCall, MessageAuthorType, MessageContentType, Project, ProjectComponent, Work,
    WorkMessage,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Get a unique identifier for test data
fn get_unique_id(_prefix: &str) -> i64 {
    COUNTER.fetch_add(1, Ordering::SeqCst) as i64
}

/// Test data generator for creating consistent test fixtures
pub struct TestDataGenerator;

#[allow(dead_code)]
impl TestDataGenerator {
    /// Create a test project with default values
    pub fn create_project(name: Option<&str>, path: Option<&str>) -> Project {
        let name = name.unwrap_or("test-project").to_string();
        let path = path.unwrap_or("/tmp/test-project").to_string();

        Project {
            id: get_unique_id("project"),
            name,
            path,
            description: None,
            parent_id: None,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a test project with custom parameters
    pub fn create_project_custom(
        name: &str,
        path: &str,
        description: Option<&str>,
        parent_id: Option<i64>,
    ) -> Project {
        Project {
            id: get_unique_id("project"),
            name: name.to_string(),
            path: path.to_string(),
            description: description.map(|s| s.to_string()),
            parent_id,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a test work session
    pub fn create_work(title: Option<&str>, project_id: Option<i64>) -> Work {
        let title = title.unwrap_or("Test Work Session").to_string();

        Work {
            id: get_unique_id("work"),
            title,
            project_id,
            tool_name: Some("test-tool".to_string()),
            model: Some("gpt-5".to_string()),
            status: "active".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a test work message
    pub fn create_work_message(
        work_id: i64,
        content: &str,
        author_type: MessageAuthorType,
        sequence_order: i32,
    ) -> WorkMessage {
        WorkMessage {
            id: get_unique_id("message"),
            work_id,
            content: content.to_string(),
            content_type: MessageContentType::Text,
            author_type,
            author_id: Some(get_unique_id("author").to_string()),
            sequence_order,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a test AI session
    pub fn create_ai_session(work_id: i64, message_id: i64, tool_name: &str) -> AiSession {
        AiSession {
            id: get_unique_id("ai-session"),
            work_id,
            message_id,
            tool_name: tool_name.to_string(),
            status: "running".to_string(),
            project_context: Some("test context".to_string()),
            started_at: chrono::Utc::now().timestamp(),
            ended_at: None,
        }
    }

    /// Create a test AI session output
    pub fn create_ai_session_output(session_id: i64, content: &str) -> AiSessionOutput {
        AiSessionOutput {
            id: 1, // Auto-increment in DB
            session_id,
            content: content.to_string(),
            created_at: chrono::Utc::now().timestamp(),
            role: Some("assistant".to_string()),
            model: Some("test-model".to_string()),
        }
    }

    /// Create a test AI session result
    pub fn create_ai_session_result(session_id: i64, response_message_id: i64) -> AiSessionResult {
        AiSessionResult {
            id: get_unique_id("ai-result"),
            session_id,
            response_message_id,
            status: "completed".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            completed_at: Some(chrono::Utc::now().timestamp()),
        }
    }

    /// Create a test LLM agent session
    pub fn create_llm_agent_session(work_id: i64, provider: &str, model: &str) -> LlmAgentSession {
        LlmAgentSession {
            id: get_unique_id("llm-session"),
            work_id,
            provider: provider.to_string(),
            model: model.to_string(),
            status: "running".to_string(),
            system_prompt: Some("You are a helpful assistant.".to_string()),
            started_at: chrono::Utc::now().timestamp(),
            ended_at: None,
        }
    }

    /// Create a test LLM agent message
    pub fn create_llm_agent_message(session_id: i64, role: &str, content: &str) -> LlmAgentMessage {
        LlmAgentMessage {
            id: 1, // Auto-increment in DB
            session_id,
            role: role.to_string(),
            content: content.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a test LLM agent tool call
    pub fn create_llm_agent_tool_call(
        session_id: i64,
        tool_name: &str,
        request: serde_json::Value,
    ) -> LlmAgentToolCall {
        LlmAgentToolCall {
            id: 1, // Auto-increment in DB
            session_id,
            message_id: None,
            tool_name: tool_name.to_string(),
            request,
            response: None,
            status: "pending".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            completed_at: None,
            execution_time_ms: None,
            progress_updates: None,
            error_details: None,
        }
    }

    /// Create a test project component
    pub fn create_project_component(
        project_id: i64,
        name: &str,
        path: &str,
        language: &str,
    ) -> ProjectComponent {
        ProjectComponent {
            id: get_unique_id("component"),
            project_id,
            name: name.to_string(),
            path: path.to_string(),
            language: language.to_string(),
            framework: Some("test-framework".to_string()),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a batch of test projects
    pub fn create_projects(count: usize) -> Vec<Project> {
        (0..count)
            .map(|i| {
                Self::create_project_custom(
                    &format!("test-project-{}", i),
                    &format!("/tmp/test-project-{}", i),
                    None,
                    None,
                )
            })
            .collect()
    }

    /// Create a complete test scenario with project, work, and messages
    pub fn create_complete_scenario() -> (Project, Work, Vec<WorkMessage>) {
        let project = Self::create_project(Some("scenario-project"), Some("/tmp/scenario-project"));
        let work = Self::create_work(Some("Scenario Work Session"), Some(project.id));

        let messages = vec![
            Self::create_work_message(
                work.id,
                "Hello, I need help with my Rust project",
                MessageAuthorType::User,
                0,
            ),
            Self::create_work_message(
                work.id,
                "I'll help you with your Rust project. What specific issue are you facing?",
                MessageAuthorType::Ai,
                1,
            ),
            Self::create_work_message(
                work.id,
                "I need to add error handling to my API endpoints",
                MessageAuthorType::User,
                2,
            ),
        ];

        (project, work, messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_ids() {
        let project1 = TestDataGenerator::create_project(None, None);
        let project2 = TestDataGenerator::create_project(None, None);

        assert_ne!(project1.id, project2.id);
        assert!(project1.id > 0);
        assert!(project2.id > 0);
    }

    #[test]
    fn test_project_creation() {
        let project =
            TestDataGenerator::create_project(Some("my-test-project"), Some("/tmp/my-test"));

        assert_eq!(project.name, "my-test-project");
        assert_eq!(project.path, "/tmp/my-test");
        assert_eq!(project.description, None);
        assert_eq!(project.parent_id, None);
        assert!(project.id > 0);
    }

    #[test]
    fn test_work_creation() {
        let work = TestDataGenerator::create_work(Some("My Work"), None);

        assert_eq!(work.title, "My Work");
        assert_eq!(work.status, "active");
        assert_eq!(work.tool_name, Some("test-tool".to_string()));
        assert!(work.id > 0);
    }

    #[test]
    fn test_work_message_creation() {
        let work = TestDataGenerator::create_work(None, None);
        let message = TestDataGenerator::create_work_message(
            work.id,
            "Test message",
            MessageAuthorType::User,
            0,
        );

        assert_eq!(message.work_id, work.id);
        assert_eq!(message.content, "Test message");
        assert!(matches!(message.author_type, MessageAuthorType::User));
        assert_eq!(message.sequence_order, 0);
        assert!(message.id > 0);
    }

    #[test]
    fn test_ai_session_creation() {
        let work = TestDataGenerator::create_work(None, None);
        let message =
            TestDataGenerator::create_work_message(work.id, "Test", MessageAuthorType::User, 0);
        let ai_session = TestDataGenerator::create_ai_session(work.id, message.id, "test-tool");

        assert_eq!(ai_session.work_id, work.id);
        assert_eq!(ai_session.message_id, message.id);
        assert_eq!(ai_session.tool_name, "test-tool");
        assert_eq!(ai_session.status, "running");
        assert!(ai_session.id > 0);
    }

    #[test]
    fn test_batch_project_creation() {
        let projects = TestDataGenerator::create_projects(3);

        assert_eq!(projects.len(), 3);
        assert_eq!(projects[0].name, "test-project-0");
        assert_eq!(projects[1].name, "test-project-1");
        assert_eq!(projects[2].name, "test-project-2");

        // All should have unique IDs
        let ids: std::collections::HashSet<_> = projects.iter().map(|p| &p.id).collect();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_complete_scenario() {
        let (project, work, messages) = TestDataGenerator::create_complete_scenario();

        assert_eq!(project.name, "scenario-project");
        assert_eq!(work.title, "Scenario Work Session");
        assert_eq!(messages.len(), 3);

        // Check message sequence
        assert_eq!(messages[0].sequence_order, 0);
        assert_eq!(messages[1].sequence_order, 1);
        assert_eq!(messages[2].sequence_order, 2);

        // Check author types alternate
        assert!(matches!(messages[0].author_type, MessageAuthorType::User));
        assert!(matches!(messages[1].author_type, MessageAuthorType::Ai));
        assert!(matches!(messages[2].author_type, MessageAuthorType::User));
    }
}
