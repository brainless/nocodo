use actix_web::{test, web};
use serde_json::json;
use std::fs;
use std::path::Path;

use nocodo_manager::models::{
    CreateProjectRequest, CreateWorkRequest, CreateAiSessionRequest,
    CreateLlmAgentSessionRequest, FileCreateRequest, FileUpdateRequest,
    MessageAuthorType, MessageContentType,
};

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_multi_step_ai_development_workflow() {
    let test_app = TestApp::new().await;

    // Phase 1: Project Initialization
    let project_temp_dir = test_app.test_config().projects_dir().join("complex-ai-project");
    let create_project_req = CreateProjectRequest {
        name: "complex-ai-project".to_string(),
        path: Some(project_temp_dir.to_string_lossy().to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_project_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project_id = body["project"]["id"].as_str().unwrap();

    // Phase 2: Development Session Setup
    let work_req = CreateWorkRequest {
        title: "Complex AI Development Workflow".to_string(),
        project_id: Some(project_id.to_string()),
        tool_name: Some("llm_agent".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/works")
        .set_json(&work_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let work_id = body["work"]["id"].as_str().unwrap();

    // Phase 3: AI Interaction - Initial Request
    let initial_message = json!({
        "content": "I need to build a REST API with user authentication, database integration, and proper error handling. Please help me design and implement this system.",
        "author_type": "user"
    });

    let msg_uri = format!("/api/works/{}/messages", work_id);
    let req = test::TestRequest::post()
        .uri(&msg_uri)
        .set_json(&initial_message)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Phase 4: AI Analysis and Planning
    let ai_analysis_message = json!({
        "content": "I'll help you build a comprehensive REST API. Let me break this down into components:\n\n1. Project structure and dependencies\n2. Database models and migrations\n3. Authentication system\n4. API routes and handlers\n5. Error handling and validation\n6. Testing\n\nLet me start by setting up the basic project structure.",
        "author_type": "ai"
    });

    let req = test::TestRequest::post()
        .uri(&msg_uri)
        .set_json(&ai_analysis_message)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Phase 5: LLM Agent Session Creation
    let llm_session_req = CreateLlmAgentSessionRequest {
        work_id: work_id.to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        system_prompt: Some("You are an expert Rust developer specializing in web APIs, authentication, and database design. Provide detailed, production-ready code with proper error handling and documentation.".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/llm-agent/sessions")
        .set_json(&llm_session_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let llm_session_id = body["session"]["id"].as_str().unwrap();

    // Phase 6: Multi-step Code Generation
    let code_generation_steps = vec![
        ("Cargo.toml", r#"[package]
name = "complex-ai-project"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-web = "4.0"
actix-rt = "2.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "macros"] }
bcrypt = "0.15"
jsonwebtoken = "8.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1.0"
anyhow = "1.0""#),

        ("src/main.rs", r#"use actix_web::{web, App, HttpServer, Result as ActixResult};
use std::io::Result as IoResult;

mod config;
mod database;
mod models;
mod handlers;
mod middleware;
mod errors;

#[actix_web::main]
async fn main() -> IoResult<()> {
    println!("Starting Complex AI API Server...");

    // Initialize configuration
    let config = config::Config::from_env();

    // Initialize database
    let database = database::Database::new(&config.database_url).await;

    // Create shared state
    let app_state = web::Data::new(AppState {
        database,
        config: config.clone(),
    });

    println!("Server running on http://{}", config.server_addr);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .configure(handlers::auth::config)
            .configure(handlers::users::config)
            .configure(handlers::health::config)
    })
    .bind(&config.server_addr)?
    .run()
    .await
}

#[derive(Clone)]
pub struct AppState {
    pub database: database::Database,
    pub config: config::Config,
}"#),

        ("src/models.rs", r#"use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}"#),
    ];

    // Execute code generation steps
    for (file_path, content) in code_generation_steps {
        let create_file_req = FileCreateRequest {
            project_id: project_id.to_string(),
            path: file_path.to_string(),
            content: Some(content.to_string()),
            is_directory: false,
        };

        let req = test::TestRequest::post()
            .uri("/api/files/create")
            .set_json(&create_file_req)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());
    }

    // Phase 7: Verification and Testing
    // Verify all files were created
    for (file_path, _) in &code_generation_steps {
        let read_req = json!({
            "project_id": project_id,
            "path": file_path
        });

        let req = test::TestRequest::post()
            .uri("/api/files/read")
            .set_json(&read_req)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["path"], *file_path);
        assert!(!body["content"].as_str().unwrap().is_empty());
    }

    // Phase 8: Documentation and Summary
    let summary_message = json!({
        "content": "Perfect! I've successfully created a comprehensive REST API foundation with:\n\n✅ Project structure with Cargo.toml\n✅ Main application setup with Actix Web\n✅ User models with authentication support\n✅ Database integration with SQLx\n✅ JWT token authentication\n✅ Error handling patterns\n\nThe system is now ready for you to:\n1. Implement the remaining modules (config, database, handlers)\n2. Add comprehensive tests\n3. Set up CI/CD pipeline\n4. Deploy to production\n\nWould you like me to continue with implementing any specific component?",
        "author_type": "ai"
    });

    let req = test::TestRequest::post()
        .uri(&msg_uri)
        .set_json(&summary_message)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Phase 9: Final Verification
    // Check complete conversation history
    let get_msg_req = test::TestRequest::get().uri(&msg_uri).to_request();
    let get_msg_resp = test::call_service(&test_app.service(), get_msg_req).await;
    assert!(get_msg_resp.status().is_success());

    let msg_body: serde_json::Value = test::read_body_json(get_msg_resp).await;
    let messages = msg_body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3); // Initial user message + AI analysis + AI summary

    // Verify project structure
    let list_req = json!({
        "project_id": project_id,
        "path": "."
    });

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let files = body["files"].as_array().unwrap();

    // Should have Cargo.toml, src directory
    let file_names: Vec<&str> = files.iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    assert!(file_names.contains(&"Cargo.toml"));
    assert!(file_names.contains(&"src"));

    // Verify src directory contents
    let src_list_req = json!({
        "project_id": project_id,
        "path": "src"
    });

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&src_list_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let src_files = body["files"].as_array().unwrap();

    let src_file_names: Vec<&str> = src_files.iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    assert!(src_file_names.contains(&"main.rs"));
    assert!(src_file_names.contains(&"models.rs"));
}

#[actix_rt::test]
async fn test_refactoring_and_code_improvement_workflow() {
    let test_app = TestApp::new().await;

    // Create project with initial code
    let project = TestDataGenerator::create_project(Some("refactor-project"), Some("/tmp/refactor-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // Create initial (suboptimal) code
    let initial_code = r#"fn calculate_total(items: Vec<i32>) -> i32 {
    let mut total = 0;
    for item in items {
        total = total + item;
    }
    total
}

fn main() {
    let numbers = vec![1, 2, 3, 4, 5];
    let result = calculate_total(numbers);
    println!("Total: {}", result);
}"#;

    fs::write(Path::new(&project.path).join("src").join("main.rs"), initial_code).unwrap();

    // Create work session for refactoring
    let work = TestDataGenerator::create_work(Some("Code Refactoring Session"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // Add refactoring request
    let refactor_message = TestDataGenerator::create_work_message(
        &work.id,
        "Please refactor this code to be more idiomatic Rust with proper error handling, documentation, and performance improvements.",
        MessageAuthorType::User,
        0,
    );
    test_app.db().create_work_message(&refactor_message).unwrap();

    // Create AI session
    let ai_session = TestDataGenerator::create_ai_session(&work.id, &refactor_message.id, "llm_agent");
    test_app.db().create_ai_session(&ai_session).unwrap();

    // Create LLM agent session
    let llm_session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
    test_app.db().create_llm_agent_session(&llm_session).unwrap();

    // AI analysis of current code
    let analysis_message = TestDataGenerator::create_work_message(
        &work.id,
        "I can see several areas for improvement in this code. Let me refactor it to be more idiomatic Rust.",
        MessageAuthorType::Ai,
        1,
    );
    test_app.db().create_work_message(&analysis_message).unwrap();

    // Refactored code
    let refactored_code = r#"//! Simple calculator demonstrating idiomatic Rust patterns
//!
//! This module provides functionality for calculating totals with proper
//! error handling and performance optimizations.

use std::iter::Sum;

/// Calculate the total of a collection of numbers
///
/// # Examples
///
/// ```
/// use refactor_project::calculate_total;
///
/// let numbers = vec![1, 2, 3, 4, 5];
/// assert_eq!(calculate_total(&numbers), 15);
/// ```
pub fn calculate_total<T>(items: &[T]) -> T
where
    T: Sum + Copy,
{
    items.iter().copied().sum()
}

/// Calculate total with overflow checking
///
/// # Examples
///
/// ```
/// use refactor_project::calculate_total_checked;
///
/// let numbers = vec![1i32, 2, 3, i32::MAX];
/// assert!(calculate_total_checked(&numbers).is_err());
/// ```
pub fn calculate_total_checked(items: &[i32]) -> Result<i32, &'static str> {
    items.iter().try_fold(0i32, |acc, &item| {
        acc.checked_add(item).ok_or("Integer overflow occurred")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_total() {
        let numbers = vec![1, 2, 3, 4, 5];
        assert_eq!(calculate_total(&numbers), 15);
    }

    #[test]
    fn test_calculate_total_empty() {
        let numbers: Vec<i32> = vec![];
        assert_eq!(calculate_total(&numbers), 0);
    }

    #[test]
    fn test_calculate_total_single_item() {
        let numbers = vec![42];
        assert_eq!(calculate_total(&numbers), 42);
    }

    #[test]
    fn test_calculate_total_checked() {
        let numbers = vec![1, 2, 3, 4, 5];
        assert_eq!(calculate_total_checked(&numbers), Ok(15));
    }

    #[test]
    fn test_calculate_total_checked_overflow() {
        let numbers = vec![i32::MAX, 1];
        assert!(calculate_total_checked(&numbers).is_err());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let numbers = vec![1, 2, 3, 4, 5];

    match calculate_total_checked(&numbers) {
        Ok(result) => {
            println!("Total: {}", result);
            println!("Calculation completed successfully!");
            Ok(())
        }
        Err(e) => {
            eprintln!("Error calculating total: {}", e);
            Err(e.into())
        }
    }
}"#;

    // Update the file with refactored code
    let update_req = FileUpdateRequest {
        project_id: project.id.clone(),
        content: refactored_code.to_string(),
    };

    let req = test::TestRequest::put()
        .uri("/api/files/src/main.rs")
        .set_json(&update_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // AI summary of improvements
    let summary_message = TestDataGenerator::create_work_message(
        &work.id,
        "I've successfully refactored your code with the following improvements:\n\n✅ **Idiomatic Rust patterns**: Used iterators and generics\n✅ **Error handling**: Added overflow checking with Result types\n✅ **Documentation**: Comprehensive docs with examples\n✅ **Performance**: More efficient iterator-based summation\n✅ **Testing**: Added comprehensive unit tests\n✅ **Type safety**: Generic implementation for better reusability\n\nThe refactored code is now production-ready with proper error handling and follows Rust best practices!",
        MessageAuthorType::Ai,
        2,
    );
    test_app.db().create_work_message(&summary_message).unwrap();

    // Verify the refactored code
    let read_req = json!({
        "project_id": project.id,
        "path": "src/main.rs"
    });

    let req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let content = body["content"].as_str().unwrap();

    // Verify key improvements are present
    assert!(content.contains("#[cfg(test)]"), "Should have test module");
    assert!(content.contains("Result<"), "Should have error handling");
    assert!(content.contains("///"), "Should have documentation");
    assert!(content.contains("iter().copied().sum()"), "Should use idiomatic iterator patterns");
    assert!(content.contains("checked_add"), "Should have overflow checking");
    assert!(content.contains("Box<dyn std::error::Error>"), "Should have proper error types");

    // Verify conversation history
    let messages = test_app.db().get_work_messages(&work.id).unwrap();
    assert_eq!(messages.len(), 3);

    // Verify LLM agent session was used
    let llm_sessions = test_app.db().get_llm_agent_sessions_by_work(&work.id).unwrap();
    assert_eq!(llm_sessions.len(), 1);
}

#[actix_rt::test]
async fn test_debugging_and_troubleshooting_workflow() {
    let test_app = TestApp::new().await;

    // Create project with buggy code
    let project = TestDataGenerator::create_project(Some("debug-project"), Some("/tmp/debug-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // Create code with intentional bugs
    let buggy_code = r#"fn divide_numbers(a: f64, b: f64) -> f64 {
    a / b  // Bug: No division by zero check
}

fn process_list(items: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for item in items {
        let processed = format!("Processed: {}", item);
        result.push(processed);
    }
    result  // Bug: Unnecessary variable, can return directly
}

fn find_max(numbers: &[i32]) -> Option<i32> {
    if numbers.is_empty() {
        return None;
    }

    let mut max = numbers[0];
    for &num in &numbers[1..] {
        if num > max {
            max = num;
        }
    }
    Some(max)  // Bug: Logic error - should be >= for proper max finding
}

fn main() {
    // Test division
    println!("Division result: {}", divide_numbers(10.0, 0.0));

    // Test list processing
    let items = vec!["hello".to_string(), "world".to_string()];
    let processed = process_list(items);
    println!("Processed: {:?}", processed);

    // Test max finding
    let numbers = vec![1, 5, 3, 9, 2];
    if let Some(max) = find_max(&numbers) {
        println!("Max: {}", max);
    }
}"#;

    fs::write(Path::new(&project.path).join("src").join("main.rs"), buggy_code).unwrap();

    // Create debugging work session
    let work = TestDataGenerator::create_work(Some("Debugging Session"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // User reports bugs
    let bug_report = TestDataGenerator::create_work_message(
        &work.id,
        "I'm getting strange behavior in my code. The division by zero doesn't seem to be handled, and there might be issues with the max-finding logic. Can you help me debug and fix these issues?",
        MessageAuthorType::User,
        0,
    );
    test_app.db().create_work_message(&bug_report).unwrap();

    // AI debugging analysis
    let debug_analysis = TestDataGenerator::create_work_message(
        &work.id,
        "I can see several issues in your code that need to be addressed. Let me analyze and fix them systematically.",
        MessageAuthorType::Ai,
        1,
    );
    test_app.db().create_work_message(&debug_analysis).unwrap();

    // Create LLM agent session for debugging
    let llm_session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&llm_session).unwrap();

    // Step-by-step fixes
    let fixes = vec![
        ("Fix division by zero", r#"fn divide_numbers(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        return Err("Division by zero is not allowed".to_string());
    }
    Ok(a / b)
}"#),

        ("Optimize list processing", r#"fn process_list(items: Vec<String>) -> Vec<String> {
    items.into_iter()
        .map(|item| format!("Processed: {}", item))
        .collect()
}"#),

        ("Fix max-finding logic", r#"fn find_max(numbers: &[i32]) -> Option<i32> {
    numbers.iter().max().copied()
}"#),
    ];

    // Apply fixes
    for (fix_description, fixed_code) in fixes {
        // Read current file
        let read_req = json!({
            "project_id": project.id,
            "path": "src/main.rs"
        });

        let req = test::TestRequest::post()
            .uri("/api/files/read")
            .set_json(&read_req)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        let current_content = body["content"].as_str().unwrap();

        // Apply fix (simplified - in real scenario would use more sophisticated replacement)
        let updated_content = if fix_description.contains("division") {
            current_content.replace(
                "fn divide_numbers(a: f64, b: f64) -> f64 {\n    a / b  // Bug: No division by zero check\n}",
                &format!("fn divide_numbers(a: f64, b: f64) -> Result<f64, String> {{\n    if b == 0.0 {{\n        return Err(\"Division by zero is not allowed\".to_string());\n    }}\n    Ok(a / b)\n}}")
            )
        } else if fix_description.contains("list processing") {
            current_content.replace(
                "fn process_list(items: Vec<String>) -> Vec<String> {\n    let mut result = Vec::new();\n    for item in items {\n        let processed = format!(\"Processed: {}\", item);\n        result.push(processed);\n    }\n    result  // Bug: Unnecessary variable, can return directly\n}",
                "fn process_list(items: Vec<String>) -> Vec<String> {\n    items.into_iter()\n        .map(|item| format!(\"Processed: {}\", item))\n        .collect()\n}"
            )
        } else if fix_description.contains("max-finding") {
            current_content.replace(
                "fn find_max(numbers: &[i32]) -> Option<i32> {\n    if numbers.is_empty() {\n        return None;\n    }\n\n    let mut max = numbers[0];\n    for &num in &numbers[1..] {\n        if num > max {\n            max = num;\n        }\n    }\n    Some(max)  // Bug: Logic error - should be >= for proper max finding\n}",
                "fn find_max(numbers: &[i32]) -> Option<i32> {\n    numbers.iter().max().copied()\n}"
            )
        } else {
            current_content.to_string()
        };

        // Update file
        let update_req = FileUpdateRequest {
            project_id: project.id.clone(),
            content: updated_content,
        };

        let req = test::TestRequest::put()
            .uri("/api/files/src/main.rs")
            .set_json(&update_req)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());
    }

    // Update main function to handle new error types
    let main_fix = r#"fn main() {
    // Test division with error handling
    match divide_numbers(10.0, 0.0) {
        Ok(result) => println!("Division result: {}", result),
        Err(e) => println!("Division error: {}", e),
    }

    // Test list processing
    let items = vec!["hello".to_string(), "world".to_string()];
    let processed = process_list(items);
    println!("Processed: {:?}", processed);

    // Test max finding
    let numbers = vec![1, 5, 3, 9, 2];
    if let Some(max) = find_max(&numbers) {
        println!("Max: {}", max);
    }

    // Test edge cases
    let empty: Vec<i32> = vec![];
    assert_eq!(find_max(&empty), None);
    println!("All tests passed!");
}"#;

    let read_req = json!({
        "project_id": project.id,
        "path": "src/main.rs"
    });

    let req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let current_content = body["content"].as_str().unwrap();

    let updated_main = current_content.replace(
        "fn main() {\n    // Test division\n    println!(\"Division result: {}\", divide_numbers(10.0, 0.0));\n\n    // Test list processing\n    let items = vec![\"hello\".to_string(), \"world\".to_string()];\n    let processed = process_list(items);\n    println!(\"Processed: {:?}\", processed);\n\n    // Test max finding\n    let numbers = vec![1, 5, 3, 9, 2];\n    if let Some(max) = find_max(&numbers) {\n        println!(\"Max: {}\", max);\n    }\n}",
        main_fix
    );

    let update_req = FileUpdateRequest {
        project_id: project.id.clone(),
        content: updated_main,
    };

    let req = test::TestRequest::put()
        .uri("/api/files/src/main.rs")
        .set_json(&update_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // AI summary of debugging session
    let debug_summary = TestDataGenerator::create_work_message(
        &work.id,
        "I've successfully debugged and fixed all the issues in your code:\n\n🔧 **Fixed division by zero**: Added proper error handling with Result types\n🔧 **Optimized list processing**: Used iterator chains instead of manual loops\n🔧 **Fixed max-finding logic**: Used built-in iterator methods for correctness\n🔧 **Updated main function**: Added proper error handling for the division operation\n\nThe code now handles edge cases properly, uses idiomatic Rust patterns, and includes comprehensive error handling. All the original bugs have been resolved!",
        MessageAuthorType::Ai,
        2,
    );
    test_app.db().create_work_message(&debug_summary).unwrap();

    // Final verification
    let final_read_req = json!({
        "project_id": project.id,
        "path": "src/main.rs"
    });

    let req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&final_read_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let final_content = body["content"].as_str().unwrap();

    // Verify all fixes are present
    assert!(final_content.contains("Result<f64, String>"), "Should have error handling for division");
    assert!(final_content.contains("if b == 0.0"), "Should check for division by zero");
    assert!(final_content.contains("into_iter()"), "Should use iterator chains");
    assert!(final_content.contains("map(|item|"), "Should use functional programming style");
    assert!(final_content.contains("iter().max()"), "Should use built-in max method");
    assert!(final_content.contains("match divide_numbers"), "Should handle division result");
    assert!(final_content.contains("All tests passed!"), "Should have success message");

    // Verify conversation history
    let messages = test_app.db().get_work_messages(&work.id).unwrap();
    assert_eq!(messages.len(), 3);

    // Verify LLM agent session
    let llm_sessions = test_app.db().get_llm_agent_sessions_by_work(&work.id).unwrap();
    assert_eq!(llm_sessions.len(), 1);
}

#[actix_rt::test]
async fn test_integrated_testing_workflow() {
    let test_app = TestApp::new().await;

    // Create project for testing
    let project = TestDataGenerator::create_project(Some("testing-workflow-project"), Some("/tmp/testing-workflow-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // Create a simple calculator module to test
    let calculator_code = r#"pub mod calculator {
    pub fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    pub fn subtract(a: i32, b: i32) -> i32 {
        a - b
    }

    pub fn multiply(a: i32, b: i32) -> i32 {
        a * b
    }

    pub fn divide(a: i32, b: i32) -> Result<i32, String> {
        if b == 0 {
            return Err("Division by zero".to_string());
        }
        Ok(a / b)
    }
}

#[cfg(test)]
mod tests {
    use super::calculator::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-1, 1), 0);
        assert_eq!(add(0, 0), 0);
    }

    #[test]
    fn test_subtract() {
        assert_eq!(subtract(5, 3), 2);
        assert_eq!(subtract(3, 5), -2);
        assert_eq!(subtract(0, 0), 0);
    }

    #[test]
    fn test_multiply() {
        assert_eq!(multiply(2, 3), 6);
        assert_eq!(multiply(-2, 3), -6);
        assert_eq!(multiply(0, 5), 0);
    }

    #[test]
    fn test_divide() {
        assert_eq!(divide(6, 3), Ok(2));
        assert_eq!(divide(5, 2), Ok(2)); // Integer division
        assert_eq!(divide(0, 5), Ok(0));
    }

    #[test]
    fn test_divide_by_zero() {
        assert!(divide(5, 0).is_err());
        assert_eq!(divide(5, 0), Err("Division by zero".to_string()));
    }

    #[test]
    fn test_edge_cases() {
        // Test with i32::MAX and i32::MIN
        assert_eq!(add(i32::MAX, 0), i32::MAX);
        assert_eq!(multiply(1, i32::MAX), i32::MAX);
    }
}"#;

    fs::write(Path::new(&project.path).join("src").join("lib.rs"), calculator_code).unwrap();

    // Create testing work session
    let work = TestDataGenerator::create_work(Some("Integrated Testing Workflow"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // User requests comprehensive testing
    let testing_request = TestDataGenerator::create_work_message(
        &work.id,
        "I've implemented a calculator module. Please help me create comprehensive tests and ensure the code is well-tested. I want to make sure all edge cases are covered and the tests are maintainable.",
        MessageAuthorType::User,
        0,
    );
    test_app.db().create_work_message(&testing_request).unwrap();

    // AI analysis and testing strategy
    let testing_strategy = TestDataGenerator::create_work_message(
        &work.id,
        "I'll help you create a comprehensive testing strategy for your calculator module. I can see you already have some basic tests, but we can improve them and add more coverage.",
        MessageAuthorType::Ai,
        1,
    );
    test_app.db().create_work_message(&testing_strategy).unwrap();

    // Create LLM agent session for testing
    let llm_session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
    test_app.db().create_llm_agent_session(&llm_session).unwrap();

    // Enhanced test file
    let enhanced_tests = r#"#[cfg(test)]
mod tests {
    use super::calculator::*;

    // Basic arithmetic tests
    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-1, 1), 0);
        assert_eq!(add(0, 0), 0);
        assert_eq!(add(i32::MAX, 0), i32::MAX);
        assert_eq!(add(i32::MIN, 0), i32::MIN);
    }

    #[test]
    fn test_subtract() {
        assert_eq!(subtract(5, 3), 2);
        assert_eq!(subtract(3, 5), -2);
        assert_eq!(subtract(0, 0), 0);
        assert_eq!(subtract(i32::MAX, 0), i32::MAX);
        assert_eq!(subtract(i32::MIN, 0), i32::MIN);
    }

    #[test]
    fn test_multiply() {
        assert_eq!(multiply(2, 3), 6);
        assert_eq!(multiply(-2, 3), -6);
        assert_eq!(multiply(0, 5), 0);
        assert_eq!(multiply(1, i32::MAX), i32::MAX);
        assert_eq!(multiply(-1, i32::MAX), i32::MIN + 1); // Two's complement edge case
    }

    #[test]
    fn test_divide() {
        assert_eq!(divide(6, 3), Ok(2));
        assert_eq!(divide(5, 2), Ok(2)); // Integer division truncates
        assert_eq!(divide(0, 5), Ok(0));
        assert_eq!(divide(-6, 3), Ok(-2));
        assert_eq!(divide(-6, -3), Ok(2));
    }

    #[test]
    fn test_divide_by_zero() {
        assert!(divide(5, 0).is_err());
        assert_eq!(divide(5, 0), Err("Division by zero".to_string()));
        assert_eq!(divide(0, 0), Err("Division by zero".to_string()));
    }

    // Property-based testing concepts
    #[test]
    fn test_add_commutative() {
        let test_cases = [(1, 2), (-1, 1), (0, 0), (100, 200)];
        for (a, b) in test_cases {
            assert_eq!(add(a, b), add(b, a), "Addition should be commutative for {} + {}", a, b);
        }
    }

    #[test]
    fn test_add_associative() {
        let test_cases = [(1, 2, 3), (-1, 1, 0), (0, 0, 0)];
        for (a, b, c) in test_cases {
            assert_eq!(add(add(a, b), c), add(a, add(b, c)), "Addition should be associative for {} + {} + {}", a, b, c);
        }
    }

    #[test]
    fn test_multiply_commutative() {
        let test_cases = [(1, 2), (-1, 1), (0, 5), (3, 4)];
        for (a, b) in test_cases {
            assert_eq!(multiply(a, b), multiply(b, a), "Multiplication should be commutative for {} * {}", a, b);
        }
    }

    #[test]
    fn test_subtract_negation() {
        let test_cases = [(5, 3), (-1, 1), (0, 0), (10, 5)];
        for (a, b) in test_cases {
            assert_eq!(subtract(a, b), add(a, multiply(b, -1)), "subtract({}, {}) should equal add({}, multiply({}, -1))", a, b, a, b);
        }
    }

    // Edge cases and boundary testing
    #[test]
    fn test_boundary_values() {
        // Test with boundary values
        assert_eq!(add(i32::MAX, 0), i32::MAX);
        assert_eq!(add(i32::MIN, 0), i32::MIN);

        assert_eq!(subtract(i32::MAX, 0), i32::MAX);
        assert_eq!(subtract(i32::MIN, 0), i32::MIN);

        assert_eq!(multiply(0, i32::MAX), 0);
        assert_eq!(multiply(0, i32::MIN), 0);
    }

    // Error handling tests
    #[test]
    fn test_divide_error_messages() {
        let result = divide(10, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }

    // Integration tests
    #[test]
    fn test_calculator_workflow() {
        // Simulate a complex calculation workflow
        let a = 10;
        let b = 5;
        let c = 2;

        // (a + b) * c - (a / b)
        let step1 = add(a, b); // 15
        let step2 = multiply(step1, c); // 30
        let step3 = divide(a, b).unwrap(); // 2
        let result = subtract(step2, step3); // 28

        assert_eq!(result, 28);

        // Alternative calculation: a + b * c - a / b
        let alt_step1 = multiply(b, c); // 10
        let alt_step2 = add(a, alt_step1); // 20
        let alt_step3 = divide(a, b).unwrap(); // 2
        let alt_result = subtract(alt_step2, alt_step3); // 18

        assert_eq!(alt_result, 18);
        assert_ne!(result, alt_result); // Different due to operator precedence
    }

    // Performance tests (basic)
    #[test]
    fn test_performance_regression() {
        use std::time::Instant;

        let start = Instant::now();

        // Perform many calculations to check for performance regressions
        let mut result = 0;
        for i in 0..10000 {
            result = add(result, i);
        }

        let duration = start.elapsed();
        assert!(duration.as_millis() < 100, "Performance regression detected: {}ms", duration.as_millis());

        // Verify correctness
        assert_eq!(result, 49995000); // Sum of 0 to 9999
    }

    // Fuzz-like testing
    #[test]
    fn test_random_inputs() {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        for _ in 0..1000 {
            let a: i32 = rng.gen_range(i32::MIN..i32::MAX);
            let b: i32 = rng.gen_range(i32::MIN..i32::MAX);

            // Test that operations don't panic
            let _ = add(a, b);
            let _ = subtract(a, b);
            let _ = multiply(a, b);

            // Division needs special handling for zero
            if b != 0 {
                let _ = divide(a, b);
            } else {
                assert!(divide(a, b).is_err());
            }
        }
    }
}"#;

    // Update the lib.rs file with enhanced tests
    let update_req = FileUpdateRequest {
        project_id: project.id.clone(),
        content: enhanced_tests.to_string(),
    };

    let req = test::TestRequest::put()
        .uri("/api/files/src/lib.rs")
        .set_json(&update_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // AI summary of testing improvements
    let testing_summary = TestDataGenerator::create_work_message(
        &work.id,
        "I've significantly enhanced your test suite with comprehensive coverage:\n\n🧪 **Basic Functionality Tests**: All arithmetic operations thoroughly tested\n🧪 **Edge Case Testing**: Boundary values, overflow conditions, zero handling\n🧪 **Property-Based Testing**: Mathematical properties (commutative, associative)\n🧪 **Error Handling Tests**: Proper error message validation\n🧪 **Integration Tests**: Complex calculation workflows\n🧪 **Performance Tests**: Regression detection for slow operations\n🧪 **Fuzz Testing**: Random input validation to prevent panics\n\nYour calculator module now has **100% test coverage** with robust error handling and comprehensive edge case coverage. The tests follow Rust best practices and will catch regressions early in development!",
        MessageAuthorType::Ai,
        2,
    );
    test_app.db().create_work_message(&testing_summary).unwrap();

    // Verify the enhanced test file
    let read_req = json!({
        "project_id": project.id,
        "path": "src/lib.rs"
    });

    let req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let content = body["content"].as_str().unwrap();

    // Verify comprehensive testing features are present
    assert!(content.contains("test_add_commutative"), "Should have property-based tests");
    assert!(content.contains("test_boundary_values"), "Should have boundary testing");
    assert!(content.contains("test_calculator_workflow"), "Should have integration tests");
    assert!(content.contains("test_performance_regression"), "Should have performance tests");
    assert!(content.contains("test_random_inputs"), "Should have fuzz-like testing");
    assert!(content.contains("assert_eq!(result, 49995000)"), "Should have correctness verification");
    assert!(content.contains("rand::Rng"), "Should use random testing");

    // Verify conversation history
    let messages = test_app.db().get_work_messages(&work.id).unwrap();
    assert_eq!(messages.len(), 3);

    // Verify LLM agent session
    let llm_sessions = test_app.db().get_llm_agent_sessions_by_work(&work.id).unwrap();
    assert_eq!(llm_sessions.len(), 1);
}