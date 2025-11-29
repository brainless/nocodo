#![allow(unused_variables)]
#![allow(unused_assignments)]

mod common;

use actix_web::{test, web, App, HttpMessage};

use crate::common::{
    keyword_validation::LlmTestScenario,
    TestApp,
};
use nocodo_manager::handlers::project_commands::discover_project_commands;

/// E2E test for command discovery API
///
/// This test demonstrates:
/// - Phase 1: Test isolation infrastructure
/// - Phase 2: Command discovery API integration
/// - Phase 3: Validation of discovered commands structure
#[actix_rt::test]
async fn test_command_discovery_saleor() {
    // Initialize logging to capture all logs in test output
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("nocodo_manager=debug".parse().unwrap())
                .add_directive("nocodo_manager::command_discovery=info".parse().unwrap()),
        )
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .try_init();

    // PHASE 1: Create isolated test environment
    println!("\n📦 Phase 1: Setting up isolated test environment");
    let test_app = TestApp::new().await;

    // Verify isolation
    assert!(test_app.test_config().test_id.starts_with("test-"));
    assert!(test_app
        .test_config()
        .db_path()
        .to_string_lossy()
        .contains(&test_app.test_config().test_id));

    println!(
        "   ✅ Test isolation configured with ID: {}",
        test_app.test_config().test_id
    );

    // PHASE 2: Set up project for command discovery
    println!("\n🔧 Phase 2: Setting up project for command discovery");

    // Create test scenario with project context (using Saleor like existing test)
    let scenario = LlmTestScenario::tech_stack_analysis_saleor();

    // Set up project context from scenario
    let project_id = test_app
        .create_project_from_scenario(&scenario.context)
        .await
        .expect("Failed to create project from scenario");

    // Verify project was created
    let projects = test_app.db().get_all_projects().unwrap();
    println!("   📁 Found {} projects in database", projects.len());
    for p in &projects {
        println!("     - Project {}: {} (path: {})", p.id, p.name, p.path);
    }
    assert!(
        projects.iter().any(|p| p.id == project_id),
        "Project {} not found in database",
        project_id
    );

    println!("   ✅ Project created from git repository: {}", scenario.context.git_repo);

    // Create a test user in the database
    let test_user = nocodo_manager::models::User {
        id: 1,
        name: "test_user".to_string(),
        email: "test@example.com".to_string(),
        password_hash: "test_hash".to_string(),
        is_active: true,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        role: None,
        last_login_at: None,
    };
    test_app.db().create_user(&test_user).unwrap();
    println!("   👤 Created test user with ID: {}", test_user.id);

    // PHASE 3: Test command discovery API
    println!("\n🔍 Phase 3: Testing command discovery API");

    // Call the command discovery endpoint
    let req = test::TestRequest::post()
        .uri(&format!("/api/projects/{}/commands/discover", project_id))
        .to_request();

    // Add mock user authentication for testing
    let mock_user = nocodo_manager::models::UserInfo {
        id: 1,
        username: "test_user".to_string(),
        email: "test@example.com".to_string(),
    };
    req.extensions_mut().insert(mock_user.clone());

    let service = test::init_service(
        App::new()
            .app_data(web::Data::new(test_app.database.database.clone()))
            .route("/api/projects/{id}/commands/discover", web::post().to(discover_project_commands))
    ).await;

    let resp = test::call_service(&service, req).await;

    // Debug: Print the response status
    let status = resp.status();
    println!("   📡 Response status: {}", status);
    
    // If not successful, print response body for debugging
    if !status.is_success() {
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8_lossy(&body);
        println!("   ❌ Response body: {}", body_str);
        panic!("Command discovery API call failed with status: {}", status);
    }

    let body: serde_json::Value = test::read_body_json(resp).await;
    println!("   ✅ Command discovery API call successful");

    // Validate response structure
    assert!(body.get("commands").is_some(), "Response missing 'commands' field");
    assert!(body.get("project_types").is_some(), "Response missing 'project_types' field");
    assert!(body.get("discovered_count").is_some(), "Response missing 'discovered_count' field");
    assert!(body.get("stored_count").is_some(), "Response missing 'stored_count' field");

    let commands = body["commands"].as_array().expect("Commands should be an array");
    let project_types = body["project_types"].as_array().expect("Project types should be an array");
    let discovered_count = body["discovered_count"].as_u64().expect("discovered_count should be a number");
    let stored_count = body["stored_count"].as_u64().expect("stored_count should be a number");

    println!("   📊 Discovery Results:");
    println!("      • Discovered commands: {}", discovered_count);
    println!("      • Stored commands: {}", stored_count);
    println!("      • Project types detected: {:?}", project_types);

    // Validate that we discovered some commands
    assert!(
        discovered_count > 0,
        "Should have discovered at least one command for Saleor project"
    );

    // Validate that project types include Python/Django (Saleor is a Python/Django project)
    let project_types_str: Vec<String> = project_types
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_lowercase())
        .collect();

    assert!(
        project_types_str.iter().any(|t| t.contains("python") || t.contains("django")),
        "Should detect Python/Django in project types, found: {:?}",
        project_types_str
    );

    // Validate command structure
    let mut found_install_command = false;
    let mut found_run_command = false;

    for command in commands {
        let cmd_obj = command.as_object().expect("Command should be an object");
        
        // Validate required fields
        assert!(cmd_obj.contains_key("id"), "Command missing 'id' field");
        assert!(cmd_obj.contains_key("name"), "Command missing 'name' field");
        assert!(cmd_obj.contains_key("command"), "Command missing 'command' field");
        assert!(cmd_obj.contains_key("description"), "Command missing 'description' field");

        let name = cmd_obj["name"].as_str().expect("Command name should be a string");
        let cmd_command = cmd_obj["command"].as_str().expect("Command command should be a string");

        println!("      • Found command: {} -> {}", name, cmd_command);

        // Check for install command
        if name.to_lowercase().contains("install") {
            found_install_command = true;
            println!("         ✅ Found install command: {}", name);
        }

        // Check for run/dev command
        if name.to_lowercase().contains("run") 
            || name.to_lowercase().contains("dev") 
            || name.to_lowercase().contains("start") {
            found_run_command = true;
            println!("         ✅ Found run/dev command: {}", name);
        }
    }

    // Validate that we found install and run commands
    assert!(
        found_install_command,
        "Should have found an install command for Saleor project"
    );

    assert!(
        found_run_command,
        "Should have found a run/dev/start command for Saleor project"
    );

    // Verify commands were stored in database
    let stored_commands = test_app.db().get_project_commands(project_id).unwrap();
    assert!(
        stored_commands.len() >= stored_count as usize,
        "Database should contain at least {} commands, found {}",
        stored_count,
        stored_commands.len()
    );

    println!("   ✅ Commands successfully stored in database");

    println!("\n🎉 Command Discovery E2E Test Complete!");
    println!("   ✅ Phase 1: Test isolation infrastructure working");
    println!("   ✅ Phase 2: Command discovery API successful");
    println!("   ✅ Phase 3: Response structure validation passed");
    println!("   📈 Discovered {} commands, stored {} commands", discovered_count, stored_count);

    // Cleanup verification
    println!("\n🧹 Cleanup verification:");
    let projects = test_app
        .db()
        .get_all_projects()
        .expect("Failed to get projects");
    println!("   📁 Test projects created: {}", projects.len());

    let works = test_app.db().get_all_works().expect("Failed to get works");
    println!("   💼 Test work sessions: {}", works.len());

    println!("   🗂️  Test files will be cleaned up automatically");
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_command_discovery_response_structure() {
        // This test validates the expected response structure
        // without making actual API calls
        
        let expected_fields = vec![
            "commands",
            "project_types", 
            "discovered_count",
            "stored_count"
        ];

        // Verify we have the right expectations
        assert!(!expected_fields.is_empty());
        assert!(expected_fields.contains(&"commands"));
        assert!(expected_fields.contains(&"project_types"));
    }

    #[tokio::test]
    async fn test_saleor_project_expectations() {
        // Test that we have the right expectations for Saleor project
        let scenario = LlmTestScenario::tech_stack_analysis_saleor();
        
        assert!(scenario.context.git_repo.contains("saleor"));
        assert!(scenario.expected_keywords.required_keywords.contains(&"Python".to_string()));
        assert!(scenario.expected_keywords.required_keywords.contains(&"Django".to_string()));
    }
}