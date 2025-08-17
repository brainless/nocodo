//! Project initialization command implementation

use std::path::PathBuf;
use crate::error::CliError;
use crate::client::{ManagerClient, CreateProjectRequest};
use tracing::{info, warn, error};

/// Initialize a new project with nocodo support
pub async fn init_project(
    template: &Option<String>,
    path: &PathBuf,
) -> Result<(), CliError> {
    println!("🚀 Initializing nocodo project at: {}", path.display());
    
    // Create Manager client
    let client = ManagerClient::new(
        "/tmp/nocodo-manager.sock".to_string(),
        None // Use default HTTP URL
    );
    
    // Check if Manager daemon is running
    if !client.check_manager_status().await? {
        return Err(CliError::Communication(
            "Manager daemon is not running. Please start nocodo-manager first.".to_string()
        ));
    }
    
    // Get available templates if no template specified
    if template.is_none() {
        println!("\n📋 Available templates:");
        match client.get_templates().await {
            Ok(templates) => {
                for template in &templates {
                    println!("  • {} - {} ({})", 
                        template.name, 
                        template.description,
                        template.language
                    );
                }
                println!("\n💡 Use --template <name> to specify a template, or continue without one for a basic project.");
            }
            Err(e) => {
                warn!("Failed to fetch templates: {}", e);
                println!("⚠️  Could not fetch available templates, proceeding with basic project.");
            }
        }
    }
    
    // Extract project name from path
    let project_name = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unnamed-project")
        .to_string();
    
    println!("\n🔧 Creating project '{}' with template: {}", 
        project_name, 
        template.as_deref().unwrap_or("default")
    );
    
    // Create project request
    let request = CreateProjectRequest {
        name: project_name.clone(),
        path: Some(path.to_string_lossy().to_string()),
        language: None,
        framework: None,
        template: template.clone(),
    };
    
    // Create project via Manager API
    match client.create_project(request).await {
        Ok(project) => {
            println!("✅ Project '{}' created successfully!", project.name);
            println!("📁 Location: {}", project.path);
            
            if let Some(language) = &project.language {
                println!("🔤 Language: {}", language);
            }
            
            if let Some(framework) = &project.framework {
                println!("🛠️  Framework: {}", framework);
            }
            
            println!("📦 Status: {}", project.status);
            
            if project.status == "initialized" {
                println!("\n🎉 Your project is ready! You can now:");
                println!("   • cd {}", project.path);
                println!("   • Start coding with AI assistance using nocodo session");
                
                // Show template-specific next steps
                if let Some(template_name) = template {
                    show_template_next_steps(template_name);
                }
            }
            
            info!("Project initialization completed: {}", project.id);
        }
        Err(e) => {
            error!("Project creation failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

fn show_template_next_steps(template_name: &str) {
    match template_name {
        "rust-web-api" => {
            println!("\n🦀 Rust Web API next steps:");
            println!("   • cargo run - Start the development server");
            println!("   • cargo watch -x run - Auto-reload on changes");
            println!("   • Visit http://localhost:8080 to test");
        }
        "node-web-app" => {
            println!("\n🟢 Node.js Web App next steps:");
            println!("   • npm install - Install dependencies");
            println!("   • npm run dev - Start development server");
            println!("   • Visit http://localhost:3000 to test");
        }
        "static-site" => {
            println!("\n🌐 Static Site next steps:");
            println!("   • Open index.html in your browser");
            println!("   • python -m http.server 8000 - Local server");
            println!("   • Visit http://localhost:8000 to test");
        }
        _ => {}
    }
}
