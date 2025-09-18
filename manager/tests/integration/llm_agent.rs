use actix_web::{test, web};
use serde_json::json;

use nocodo_manager::models::{
    CreateAiSessionRequest, CreateLlmAgentSessionRequest, LlmAgentSession,
    MessageAuthorType, MessageContentType,
};

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_create_llm_agent_session() {
    let test_app = TestApp::new().await;

    // Create a project and work session first
    let project = TestDataGenerator::create_project(Some("llm-agent-project"), Some("/tmp/llm-agent-project"));
    test_app.db().create_project(&project).unwrap();

    let work = TestDataGenerator::create_work(Some("LLM Agent Work"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    let session_request = CreateLlmAgentSessionRequest {
        work_id: work.id.clone(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        system_prompt: Some("You are a helpful coding assistant.".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/llm-agent/sessions")
        .set_json(&session_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let session = &body["session"];

    assert_eq!(session["work_id"], work.id);
    assert_eq!(session["provider"], "openai");
    assert_eq!(session["model"], "gpt-4");
    assert_eq!(session["status"], "running");
    assert_eq!(session["system_prompt"], "You are a helpful coding assistant.");
    assert!(session["id"].as_str().is_some());
    assert!(session["started_at"].as_i64().is_some());

    // Verify session was created in database
    let db_sessions = test_app.db().get_llm_agent_sessions_by_work(&work.id).unwrap();
    assert_eq!(db_sessions.len(), 1);
    assert_eq!(db_sessions[0].provider, "openai");
    assert_eq!(db_sessions[0].model, "gpt-4");
}

#[actix_rt::test]
async fn test_get_llm_agent_session() {
    let test_app = TestApp::new().await;

    // Create a session first
    let work = TestDataGenerator::create_work(Some("Get Session Test"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Get the session
    let uri = format!("/api/llm-agent/sessions/{}", session.id);
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&test_app.service(), req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let retrieved_session = &body["session"];

    assert_eq!(retrieved_session["id"], session.id);
    assert_eq!(retrieved_session["work_id"], work.id);
    assert_eq!(retrieved_session["provider"], "anthropic");
    assert_eq!(retrieved_session["model"], "claude-3");
    assert_eq!(retrieved_session["status"], "running");
}

#[actix_rt::test]
async fn test_get_llm_agent_session_not_found() {
    let test_app = TestApp::new().await;

    let req = test::TestRequest::get()
        .uri("/api/llm-agent/sessions/non-existent-id")
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "llm_agent_session_not_found");
}

#[actix_rt::test]
async fn test_update_llm_agent_session() {
    let test_app = TestApp::new().await;

    // Create a session first
    let work = TestDataGenerator::create_work(Some("Update Session Test"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-3.5-turbo");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Update the session
    let update_data = json!({
        "status": "completed",
        "model": "gpt-4-turbo"
    });

    let uri = format!("/api/llm-agent/sessions/{}", session.id);
    let req = test::TestRequest::put()
        .uri(&uri)
        .set_json(&update_data)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let updated_session = &body["session"];

    assert_eq!(updated_session["id"], session.id);
    assert_eq!(updated_session["status"], "completed");
    assert_eq!(updated_session["model"], "gpt-4-turbo");

    // Verify update in database
    let db_session = test_app.db().get_llm_agent_session(&session.id).unwrap();
    assert_eq!(db_session.status, "completed");
    assert_eq!(db_session.model, "gpt-4-turbo");
}

#[actix_rt::test]
async fn test_llm_agent_session_messages() {
    let test_app = TestApp::new().await;

    // Create a session first
    let work = TestDataGenerator::create_work(Some("Message Session Test"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Add messages to the session
    let messages = vec![
        ("user", "Hello, can you help me with Rust?"),
        ("assistant", "Yes, I'd be happy to help you with Rust! What would you like to know?"),
        ("user", "How do I create a vector in Rust?"),
        ("assistant", "To create a vector in Rust, you can use Vec::new() or the vec![] macro."),
    ];

    for (role, content) in messages {
        let message_data = json!({
            "role": role,
            "content": content
        });

        let uri = format!("/api/llm-agent/sessions/{}/messages", session.id);
        let req = test::TestRequest::post()
            .uri(&uri)
            .set_json(&message_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());
    }

    // Get messages
    let uri = format!("/api/llm-agent/sessions/{}/messages", session.id);
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&test_app.service(), req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let retrieved_messages = body["messages"].as_array().unwrap();

    assert_eq!(retrieved_messages.len(), 4);

    // Verify message order and content
    assert_eq!(retrieved_messages[0]["role"], "user");
    assert_eq!(retrieved_messages[0]["content"], "Hello, can you help me with Rust?");
    assert_eq!(retrieved_messages[1]["role"], "assistant");
    assert_eq!(retrieved_messages[1]["content"], "Yes, I'd be happy to help you with Rust! What would you like to know?");
    assert_eq!(retrieved_messages[2]["role"], "user");
    assert_eq!(retrieved_messages[2]["content"], "How do I create a vector in Rust?");
    assert_eq!(retrieved_messages[3]["role"], "assistant");
    assert_eq!(retrieved_messages[3]["content"], "To create a vector in Rust, you can use Vec::new() or the vec![] macro.");
}

#[actix_rt::test]
async fn test_llm_agent_session_with_work_context() {
    let test_app = TestApp::new().await;

    // Create project and work with messages
    let project = TestDataGenerator::create_project(Some("Context Project"), Some("/tmp/context-project"));
    test_app.db().create_project(&project).unwrap();

    let work = TestDataGenerator::create_work(Some("Context Work"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // Add some work messages
    let work_messages = vec![
        TestDataGenerator::create_work_message(&work.id, "I need help with my Rust project", MessageAuthorType::User, 0),
        TestDataGenerator::create_work_message(&work.id, "What specific help do you need?", MessageAuthorType::Ai, 1),
        TestDataGenerator::create_work_message(&work.id, "I need to add error handling", MessageAuthorType::User, 2),
    ];

    for message in &work_messages {
        test_app.db().create_work_message(message).unwrap();
    }

    // Create LLM agent session
    let session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3-opus");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Verify session is linked to work
    let db_session = test_app.db().get_llm_agent_session(&session.id).unwrap();
    assert_eq!(db_session.work_id, work.id);

    // Get work with history should include the LLM session
    let work_with_history = test_app.db().get_work_with_messages(&work.id).unwrap();
    assert_eq!(work_with_history.messages.len(), 3);

    // Get LLM sessions by work
    let sessions = test_app.db().get_llm_agent_sessions_by_work(&work.id).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, session.id);
}

#[actix_rt::test]
async fn test_multiple_llm_agent_sessions_per_work() {
    let test_app = TestApp::new().await;

    // Create work
    let work = TestDataGenerator::create_work(Some("Multi Session Work"), None);
    test_app.db().create_work(&work).unwrap();

    // Create multiple sessions for the same work
    let sessions_data = vec![
        ("openai", "gpt-4"),
        ("anthropic", "claude-3"),
        ("google", "gemini-pro"),
    ];

    let mut created_sessions = Vec::new();

    for (provider, model) in sessions_data {
        let session = TestDataGenerator::create_llm_agent_session(&work.id, provider, model);
        test_app.db().create_llm_agent_session(&session).unwrap();
        created_sessions.push(session);
    }

    // Verify all sessions were created
    let db_sessions = test_app.db().get_llm_agent_sessions_by_work(&work.id).unwrap();
    assert_eq!(db_sessions.len(), 3);

    // Verify each session has correct provider and model
    let providers: Vec<&str> = db_sessions.iter().map(|s| s.provider.as_str()).collect();
    let models: Vec<&str> = db_sessions.iter().map(|s| s.model.as_str()).collect();

    assert!(providers.contains(&"openai"));
    assert!(providers.contains(&"anthropic"));
    assert!(providers.contains(&"google"));

    assert!(models.contains(&"gpt-4"));
    assert!(models.contains(&"claude-3"));
    assert!(models.contains(&"gemini-pro"));
}

#[actix_rt::test]
async fn test_llm_agent_session_status_transitions() {
    let test_app = TestApp::new().await;

    // Create session
    let work = TestDataGenerator::create_work(Some("Status Transition Test"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Test status transitions
    let statuses = vec!["running", "processing", "completed", "failed"];

    for status in statuses {
        let update_data = json!({
            "status": status
        });

        let uri = format!("/api/llm-agent/sessions/{}", session.id);
        let req = test::TestRequest::put()
            .uri(&uri)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["session"]["status"], status);

        // Verify in database
        let db_session = test_app.db().get_llm_agent_session(&session.id).unwrap();
        assert_eq!(db_session.status, status);
    }
}

#[actix_rt::test]
async fn test_llm_agent_session_with_system_prompt() {
    let test_app = TestApp::new().await;

    // Create session with detailed system prompt
    let work = TestDataGenerator::create_work(Some("System Prompt Test"), None);
    test_app.db().create_work(&work).unwrap();

    let detailed_prompt = r#"You are an expert Rust developer with deep knowledge of:
- Systems programming concepts
- Memory management and ownership
- Async programming with tokio
- Web development with actix-web
- Database integration with diesel

Always provide:
1. Clear, concise explanations
2. Working code examples
3. Best practices and performance considerations
4. Error handling patterns

Be helpful, patient, and encouraging to developers of all skill levels."#;

    let session_request = CreateLlmAgentSessionRequest {
        work_id: work.id.clone(),
        provider: "anthropic".to_string(),
        model: "claude-3-opus".to_string(),
        system_prompt: Some(detailed_prompt.to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/llm-agent/sessions")
        .set_json(&session_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let session = &body["session"];

    assert_eq!(session["system_prompt"], detailed_prompt);

    // Verify in database
    let db_session = test_app.db().get_llm_agent_session(session["id"].as_str().unwrap()).unwrap();
    assert_eq!(db_session.system_prompt, Some(detailed_prompt.to_string()));
}

#[actix_rt::test]
async fn test_llm_agent_session_conversation_flow() {
    let test_app = TestApp::new().await;

    // Create session
    let work = TestApp::new().await;
    let project = TestDataGenerator::create_project(Some("Conversation Project"), Some("/tmp/conversation-project"));
    work.db().create_project(&project).unwrap();

    let work_session = TestDataGenerator::create_work(Some("Conversation Work"), Some(&project.id));
    work.db().create_work(&work_session).unwrap();

    let llm_session = TestDataGenerator::create_llm_agent_session(&work_session.id, "openai", "gpt-4");
    work.db().create_llm_agent_session(&llm_session).unwrap();

    // Simulate a conversation flow
    let conversation = vec![
        ("system", "You are a Rust expert. Provide helpful, accurate responses."),
        ("user", "How do I implement a simple HTTP server in Rust?"),
        ("assistant", "You can use the actix-web framework. Here's a basic example:\n\n```rust\nuse actix_web::{web, App, HttpServer, Result};\n\nasync fn hello() -> Result<String> {\n    Ok(\"Hello world!\".to_string())\n}\n\n#[actix_web::main]\nasync fn main() -> std::io::Result<()> {\n    HttpServer::new(|| {\n        App::new()\n            .route(\"/\", web::get().to(hello))\n    })\n    .bind(\"127.0.0.1:8080\")?\n    .run()\n    .await\n}\n```"),
        ("user", "How do I add JSON request/response handling?"),
        ("assistant", "You can use serde for JSON serialization. Here's how to handle JSON requests and responses:\n\n```rust\nuse actix_web::{web, App, HttpServer, Result};\nuse serde::{Deserialize, Serialize};\n\n#[derive(Serialize, Deserialize)]\nstruct User {\n    name: String,\n    email: String,\n}\n\nasync fn create_user(user: web::Json<User>) -> Result<web::Json<User>> {\n    Ok(user)\n}\n\n#[actix_web::main]\nasync fn main() -> std::io::Result<()> {\n    HttpServer::new(|| {\n        App::new()\n            .route(\"/users\", web::post().to(create_user))\n    })\n    .bind(\"127.0.0.1:8080\")?\n    .run()\n    .await\n}\n```"),
    ];

    for (role, content) in conversation {
        let message_data = json!({
            "role": role,
            "content": content
        });

        let uri = format!("/api/llm-agent/sessions/{}/messages", llm_session.id);
        let req = test::TestRequest::post()
            .uri(&uri)
            .set_json(&message_data)
            .to_request();

        let resp = test::call_service(&work.service(), req).await;
        assert!(resp.status().is_success());
    }

    // Verify conversation was stored
    let messages = work.db().get_llm_agent_messages(&llm_session.id).unwrap();
    assert_eq!(messages.len(), 4);

    // Verify message order
    assert_eq!(messages[0].role, "system");
    assert_eq!(messages[1].role, "user");
    assert_eq!(messages[2].role, "assistant");
    assert_eq!(messages[3].role, "assistant");

    // Verify content contains expected code examples
    assert!(messages[1].content.contains("HTTP server"));
    assert!(messages[2].content.contains("actix-web"));
    assert!(messages[3].content.contains("serde"));
}

#[actix_rt::test]
async fn test_llm_agent_session_error_handling() {
    let test_app = TestApp::new().await;

    // Test: Create session with non-existent work
    let invalid_request = CreateLlmAgentSessionRequest {
        work_id: "non-existent-work".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        system_prompt: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/llm-agent/sessions")
        .set_json(&invalid_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Invalid work ID"));

    // Test: Invalid message role
    let work = TestDataGenerator::create_work(Some("Error Handling Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&session).unwrap();

    let invalid_message = json!({
        "role": "invalid_role",
        "content": "Test message"
    });

    let uri = format!("/api/llm-agent/sessions/{}/messages", session.id);
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_json(&invalid_message)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Invalid message role"));
}