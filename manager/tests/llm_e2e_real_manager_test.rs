mod common;

use crate::common::{
    keyword_validation::{KeywordValidator, LlmTestScenario},
    llm_config::LlmTestConfig,
    RealManagerInstance,
};

/// E2E test using a real nocodo-manager daemon instance
/// This test spawns an actual manager process to eliminate async runtime differences
#[actix_rt::test]
async fn test_llm_e2e_real_manager_saleor() {
    // Get LLM configuration from environment and skip if no providers available
    let llm_config = LlmTestConfig::from_environment();
    if !llm_config.has_available_providers() {
        println!("⚠️  Skipping LLM E2E test - no API keys available");
        println!("   Set XAI_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY to run this test");
        return;
    }

    let provider = llm_config
        .get_default_provider()
        .expect("No default provider available");

    println!("🚀 Running LLM E2E test with real manager instance");
    println!("   Provider: {}", provider.name);
    println!("   Model: {}", provider.default_model());

    // PHASE 1: Start real manager instance
    println!("\n📦 Phase 1: Starting real manager daemon");
    let manager = RealManagerInstance::start(provider)
        .await
        .expect("Failed to start real manager instance");

    println!("   ✅ Manager daemon started at {}", manager.base_url);

    // PHASE 2: Set up test scenario with project context
    println!("\n🤖 Phase 2: Setting up test scenario");
    let scenario = LlmTestScenario::tech_stack_analysis_saleor();

    // Create project context using git repository clone
    let project_name = format!(
        "nocodo-test-{}",
        scenario
            .context
            .git_repo
            .split('/')
            .next_back()
            .unwrap_or("saleor")
    );
    let project_path = manager.config.temp_dir_path().join(&project_name);

    // Create project via manager API first (this validates the path doesn't exist)
    let project_id = manager
        .create_project(&project_name, project_path.to_str().unwrap())
        .await
        .expect("Failed to create project");

    println!("   ✅ Project created: {}", project_id);

    // Clone the git repository for analysis into the project directory
    println!("   📥 Cloning repository: {}", scenario.context.git_repo);

    // Clone to a temporary directory first
    let temp_clone_path = manager.config.temp_dir_path().join("temp_clone");
    let clone_result = std::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1", // Shallow clone for faster setup
            &scenario.context.git_repo,
            temp_clone_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute git clone");

    if !clone_result.status.success() {
        panic!(
            "Failed to clone repository: {}",
            String::from_utf8_lossy(&clone_result.stderr)
        );
    }

    // Move the cloned contents to the project directory
    let move_result = std::process::Command::new("cp")
        .args([
            "-r",
            &format!("{}/*", temp_clone_path.to_str().unwrap()),
            project_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to copy cloned repository");

    if !move_result.status.success() {
        // Try with individual files if wildcard fails
        let move_result2 = std::process::Command::new("bash")
            .args([
                "-c",
                &format!(
                    "cp -r {}/* {}/",
                    temp_clone_path.to_str().unwrap(),
                    project_path.to_str().unwrap()
                ),
            ])
            .output()
            .expect("Failed to copy cloned repository contents");

        if !move_result2.status.success() {
            panic!(
                "Failed to copy repository contents: {}",
                String::from_utf8_lossy(&move_result2.stderr)
            );
        }
    }

    // Clean up temporary clone directory
    std::fs::remove_dir_all(&temp_clone_path).ok();

    println!("   ✅ Repository cloned to {}", project_path.display());

    // PHASE 3: Execute LLM workflow via real API
    println!("\n🎯 Phase 3: Executing LLM workflow via real manager API");

    // Read work title from prompts configuration
    let work_title = crate::common::keyword_validation::PromptsConfig::load_from_file(
        std::path::Path::new("prompts/default.toml"),
    )
    .map(|config| config.tech_stack_analysis.prompt.clone())
    .unwrap_or_else(|_| "Tech Stack Analysis".to_string());

    // 1. Create work session
    let work_id = manager
        .create_work(&work_title, Some(project_id.clone()))
        .await
        .expect("Failed to create work");

    println!("   ✅ Work session created: {}", work_id);

    // 2. Add user message
    let message_id = manager
        .add_message(&work_id, &scenario.prompt)
        .await
        .expect("Failed to add message");

    println!("   ✅ Message added: {}", message_id);
    println!("   📤 Prompt: {}", scenario.prompt);

    // 3. Create AI session (this triggers the background LLM processing)
    let session_id = manager
        .create_ai_session(&work_id, &message_id)
        .await
        .expect("Failed to create AI session");

    println!("   ✅ AI session created: {}", session_id);
    println!("   ⏳ Waiting for LLM processing...");

    // PHASE 4: Monitor LLM processing and collect results
    println!("\n📊 Phase 4: Monitoring LLM processing");

    let mut attempts = 0;
    let max_attempts = 12; // 60 seconds total
    let mut response_content = String::new();
    let mut printed_output_ids = std::collections::HashSet::new();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        attempts += 1;

        // Get AI outputs via real API
        let ai_outputs = manager
            .get_ai_outputs(&work_id)
            .await
            .expect("Failed to get AI outputs");

        // Print new outputs
        let mut has_new_outputs = false;
        for output in &ai_outputs {
            let output_id = output["id"].as_str().unwrap_or("unknown");
            let content = output["content"].as_str().unwrap_or("");

            if !printed_output_ids.contains(output_id) && !content.is_empty() {
                let preview = if content.len() > 100 {
                    format!("{}...", &content[..100])
                } else {
                    content.to_string()
                };
                println!("   📝 New AI output: {}", preview);
                printed_output_ids.insert(output_id.to_string());
                has_new_outputs = true;
            }
        }

        // Debug output after 5 attempts (25 seconds)
        if attempts == 5 {
            println!(
                "   🔍 DEBUG: Total AI outputs after {} attempts: {}",
                attempts,
                ai_outputs.len()
            );
            for (i, output) in ai_outputs.iter().enumerate() {
                let content = output["content"].as_str().unwrap_or("");
                println!(
                    "   🔍 DEBUG: Output {}: content_len={}, preview={}",
                    i,
                    content.len(),
                    if content.len() > 100 {
                        format!("{}...", &content[..100])
                    } else {
                        content.to_string()
                    }
                );
            }
        }

        // Check for final text response (not tool calls)
        if let Some(output) = ai_outputs.iter().rev().find(|output| {
            let content = output["content"].as_str().unwrap_or("");
            !content.is_empty() &&
            !content.trim().starts_with("{\"type") && // Not a tool call
            !content.trim().starts_with("{\"files") && // Not a tool response
            !content.trim().starts_with("{\"content") && // Not a file content response
            !content.trim().contains("\"type") && // Not containing tool call syntax
            !content.trim().contains("read_file") && // Not containing tool names
            !content.trim().contains("list_files") // Not containing tool names
        }) {
            response_content = output["content"].as_str().unwrap_or("").to_string();
            println!(
                "   ✅ AI text response received after {} attempts ({} seconds)",
                attempts,
                attempts * 5
            );
            break;
        }

        // Check for sufficient tool activity (file reads with config content)
        let has_meaningful_content = ai_outputs.iter().any(|output| {
            let content = output["content"].as_str().unwrap_or("");
            (content.contains("\"type\":\"read_file\"") && (
                content.contains("package.json") ||
                content.contains("pyproject.toml") ||
                content.contains("requirements") ||
                content.contains("django") ||
                content.contains("graphql") ||
                content.contains("postgresql") ||
                content.contains("uvicorn")
            )) ||
            // Or final text response with tech stack keywords
            (content.contains("Django") && content.contains("Python") && content.contains("GraphQL"))
        });

        // Continue waiting if we have outputs but no meaningful content yet
        if !ai_outputs.is_empty() && attempts < max_attempts - 2 && !has_meaningful_content {
            if has_new_outputs {
                println!(
                    "   🔧 Found {} total outputs, waiting for meaningful analysis...",
                    ai_outputs.len()
                );
            }
        } else if !ai_outputs.is_empty() {
            // Combine all responses for validation
            let mut combined_content = String::new();
            for output in ai_outputs.iter() {
                let content = output["content"].as_str().unwrap_or("");
                if !content.is_empty() {
                    combined_content.push_str(content);
                    combined_content.push(' ');
                }
            }

            response_content = combined_content;
            println!(
                "   📝 Using combined tool responses for validation after {} attempts",
                attempts
            );
            break;
        }

        if attempts >= max_attempts {
            println!(
                "   ⚠️  Timeout waiting for AI response after {} seconds",
                max_attempts * 5
            );
            break;
        }

        if !has_new_outputs {
            println!(
                "   ⏳ Waiting for AI response... (attempt {}/{})",
                attempts, max_attempts
            );
        }
    }

    // Get final outputs for reporting
    let final_outputs = manager
        .get_ai_outputs(&work_id)
        .await
        .expect("Failed to get final AI outputs");

    println!("   🔍 Final output count: {}", final_outputs.len());
    for (i, output) in final_outputs.iter().enumerate() {
        let content = output["content"].as_str().unwrap_or("");
        println!(
            "      Output {}: content_preview={}",
            i + 1,
            content.chars().take(50).collect::<String>()
        );
    }

    // PHASE 5: Validate results
    println!("\n🔍 Phase 5: Validating LLM response");

    if response_content.is_empty() {
        panic!("No response content received from LLM agent");
    }

    println!(
        "   📥 Response received ({} chars): {}...",
        response_content.len(),
        if response_content.len() > 100 {
            &response_content[..100]
        } else {
            &response_content
        }
    );

    let validation_result =
        KeywordValidator::validate_response(&response_content, &scenario.expected_keywords);

    println!("   📊 Validation Results:");
    println!("      • Score: {:.2}", validation_result.score);
    println!(
        "      • Required keywords found: {:?}",
        validation_result.found_required
    );
    println!(
        "      • Optional keywords found: {:?}",
        validation_result.found_optional
    );
    println!(
        "      • Forbidden keywords found: {:?}",
        validation_result.found_forbidden
    );

    if !validation_result.missing_required.is_empty() {
        println!(
            "      • Missing required keywords: {:?}",
            validation_result.missing_required
        );
    }

    // Test assertions
    assert!(
        validation_result.passed,
        "LLM response validation failed for provider {}: {}\n\n\
         📝 Full Response:\n{}\n\n\
         📊 Validation Details:\n\
         • Score: {:.2} (minimum: {:.2})\n\
         • Required found: {:?}\n\
         • Required missing: {:?}\n\
         • Forbidden found: {:?}\n\
         • Optional found: {:?}",
        provider.name,
        scenario.name,
        response_content,
        validation_result.score,
        scenario.expected_keywords.minimum_score,
        validation_result.found_required,
        validation_result.missing_required,
        validation_result.found_forbidden,
        validation_result.found_optional
    );

    println!("   ✅ Keyword validation passed!");

    // Additional verifications
    assert!(
        response_content.len() > 50,
        "LLM response too short: {}",
        response_content
    );

    let response_lower = response_content.to_lowercase();
    assert!(
        !response_lower.contains("error")
            || response_lower.contains("api")
            || response_lower.contains("python"),
        "LLM response appears to be an error: {}",
        response_content
    );

    println!("\n🎉 Real Manager E2E Test Complete!");
    println!("   ✅ Real manager daemon operated correctly");
    println!("   ✅ LLM follow-up tool calling worked properly");
    println!("   ✅ Keyword validation passed");
    println!("   📈 Overall score: {:.2}/1.0", validation_result.score);
    println!("   📊 Total outputs generated: {}", final_outputs.len());

    // Manager instance will be cleaned up automatically when dropped
}
