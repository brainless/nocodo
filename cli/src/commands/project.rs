//! Project management command implementations

use crate::{
    cli::ProjectCommands,
    client::{AddExistingProjectRequest, ManagerClient},
    commands::analyze::ProjectAnalyzer,
    error::CliError,
};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Handle project management commands
pub async fn handle_project_command(action: &ProjectCommands) -> Result<(), CliError> {
    match action {
        ProjectCommands::Add { path } => add_project(path).await,
    }
}

/// Add an existing project to the nocodo manager
async fn add_project(path: &Option<PathBuf>) -> Result<(), CliError> {
    // Determine target path - default to current directory if not specified
    let target_path = path
        .as_ref()
        .map(|p| p.as_path())
        .unwrap_or_else(|| Path::new("."));

    info!("Adding project at path: {:?}", target_path);

    // Convert to absolute path
    let absolute_path = target_path.canonicalize().map_err(|e| {
        CliError::Analysis(format!("Failed to resolve path {:?}: {}", target_path, e))
    })?;

    info!("Resolved absolute path: {:?}", absolute_path);

    // Validate directory exists and is a directory
    if !absolute_path.exists() {
        return Err(CliError::Analysis(format!(
            "Directory does not exist: {}",
            absolute_path.display()
        )));
    }

    if !absolute_path.is_dir() {
        return Err(CliError::Analysis(format!(
            "Path is not a directory: {}",
            absolute_path.display()
        )));
    }

    // Analyze the project to extract metadata
    let analyzer = ProjectAnalyzer::new();
    let analysis = analyzer.analyze(&absolute_path).await?;

    // Check if this looks like a valid project
    if analysis.file_count == 0 {
        warn!("No files found in directory - this may not be a valid project");
    }

    // Extract project name from directory name or project files
    let project_name = extract_project_name(&absolute_path, &analysis)?;
    
    // Determine primary language and framework
    let language = if !analysis.primary_language.is_empty() {
        Some(analysis.primary_language.clone())
    } else {
        None
    };

    let framework = detect_framework(&analysis);

    info!(
        "Detected project: name='{}', language={:?}, framework={:?}",
        project_name, language, framework
    );

    // Create manager client
    let socket_path = std::env::var("NOCODO_SOCKET_PATH")
        .unwrap_or_else(|_| "/var/run/nocodo/manager.sock".to_string());
    let manager_url = std::env::var("NOCODO_MANAGER_URL").ok();
    let client = ManagerClient::new(socket_path, manager_url);

    // Check if manager is accessible
    if !client.check_manager_status().await? {
        return Err(CliError::Communication(
            "Manager daemon is not running or not accessible. Please start nocodo-manager first."
                .to_string(),
        ));
    }

    // Check for existing projects and uniqueness
    let absolute_path_str = absolute_path.to_string_lossy().to_string();
    if let Ok(_existing_project) = client.get_project_by_path(absolute_path_str.clone()).await {
        return Err(CliError::Analysis(format!(
            "Project at path {} already exists in manager",
            absolute_path.display()
        )));
    }

    // Check if this path is inside an existing project
    // We need to check all existing projects to see if this path is a subdirectory of any of them
    if let Err(_) = validate_not_inside_existing_project(&client, &absolute_path).await {
        return Err(CliError::Analysis(
            "Cannot add a folder that is inside an existing project as another project".to_string(),
        ));
    }

    // Create add existing project request
    let add_existing_request = AddExistingProjectRequest {
        name: project_name.clone(),
        path: absolute_path_str,
        language,
        framework,
    };

    // Send request to manager
    info!("Adding existing project via API...");
    let created_project = client.add_existing_project(add_existing_request).await?;

    // Success output
    println!("✓ Detected {} project: {}", 
             created_project.language.as_deref().unwrap_or("unknown"), 
             created_project.name);
    println!("✓ Added project \"{}\" at {}", 
             created_project.name, 
             created_project.path);

    Ok(())
}

/// Extract project name from path and analysis
fn extract_project_name(path: &Path, analysis: &crate::commands::analyze::ProjectAnalysis) -> Result<String, CliError> {
    // Try to get name from Rust project
    if let Some(rust_info) = &analysis.rust_info {
        if let Some(package_name) = &rust_info.package_name {
            return Ok(package_name.clone());
        }
    }

    // Try to get name from Node.js project
    if let Some(node_info) = &analysis.node_info {
        if let Some(package_name) = &node_info.package_name {
            return Ok(package_name.clone());
        }
    }

    // Fall back to directory name
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| CliError::Analysis("Could not determine project name from path".to_string()))
}

/// Detect framework from project analysis
fn detect_framework(analysis: &crate::commands::analyze::ProjectAnalysis) -> Option<String> {
    // Check if it's a Rust project
    if let Some(_rust_info) = &analysis.rust_info {
        // Could check dependencies to detect specific frameworks like Axum, Actix, etc.
        // For now, just return None to let the manager determine framework
        return None;
    }

    // Check if it's a Node.js project  
    if let Some(node_info) = &analysis.node_info {
        // Check common framework dependencies
        if node_info.dependencies.contains(&"express".to_string()) {
            return Some("express".to_string());
        }
        if node_info.dependencies.contains(&"react".to_string()) {
            return Some("react".to_string());
        }
        if node_info.dependencies.contains(&"vue".to_string()) {
            return Some("vue".to_string());
        }
        if node_info.dependencies.contains(&"angular".to_string()) {
            return Some("angular".to_string());
        }
        if node_info.dependencies.contains(&"next".to_string()) {
            return Some("nextjs".to_string());
        }
    }

    None
}

/// Validate that the path is not inside an existing project
/// This is a placeholder - we'd need to implement a way to get all projects from the manager
/// For now, we'll just do a basic check
async fn validate_not_inside_existing_project(
    _client: &ManagerClient,
    path: &Path,
) -> Result<(), CliError> {
    // TODO: Implement proper validation by fetching all existing projects from manager
    // and checking if the given path is a subdirectory of any existing project
    // For now, we'll do a simple check by looking for common project markers in parent directories
    
    let mut current_path = path.parent();
    while let Some(parent) = current_path {
        // Check for common project root markers in parent directories
        if parent.join("Cargo.toml").exists() 
            || parent.join("package.json").exists()
            || parent.join(".git").exists() {
            
            // This could be a parent project, but we need more sophisticated checking
            // For MVP, we'll allow this and let the manager handle the uniqueness validation
            warn!("Found potential parent project at: {:?}", parent);
        }
        
        current_path = parent.parent();
        
        // Don't go beyond reasonable depth
        if parent.components().count() < 2 {
            break;
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::analyze::{ProjectAnalysis, ProjectType, RustProjectInfo, NodeProjectInfo, PackageManager};
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_extract_project_name_from_rust_project() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        
        let rust_info = RustProjectInfo {
            cargo_toml_path: path.join("Cargo.toml"),
            is_workspace: false,
            workspace_members: vec![],
            package_name: Some("my-rust-app".to_string()),
            dependencies: vec![],
            dev_dependencies: vec![],
            bin_targets: vec![],
            lib_target: false,
        };
        
        let analysis = ProjectAnalysis {
            project_path: path.to_path_buf(),
            project_type: ProjectType::RustApplication,
            rust_info: Some(rust_info),
            node_info: None,
            node_projects: vec![],
            file_count: 5,
            total_lines: 100,
            primary_language: "rust".to_string(),
            languages: HashMap::new(),
            recommendations: vec![],
        };
        
        let name = extract_project_name(path, &analysis).unwrap();
        assert_eq!(name, "my-rust-app");
    }

    #[test]
    fn test_extract_project_name_from_node_project() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        
        let node_info = NodeProjectInfo {
            package_json_path: path.join("package.json"),
            package_manager: PackageManager::Npm,
            package_name: Some("my-node-app".to_string()),
            version: Some("1.0.0".to_string()),
            dependencies: vec![],
            dev_dependencies: vec![],
            scripts: HashMap::new(),
            main_entry: Some("index.js".to_string()),
            has_typescript: false,
        };
        
        let analysis = ProjectAnalysis {
            project_path: path.to_path_buf(),
            project_type: ProjectType::NodeApplication,
            rust_info: None,
            node_info: Some(node_info.clone()),
            node_projects: vec![node_info],
            file_count: 5,
            total_lines: 100,
            primary_language: "javascript".to_string(),
            languages: HashMap::new(),
            recommendations: vec![],
        };
        
        let name = extract_project_name(path, &analysis).unwrap();
        assert_eq!(name, "my-node-app");
    }

    #[test]
    fn test_extract_project_name_fallback_to_directory_name() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        
        // Create a subdirectory with a specific name
        let project_path = path.join("my-project");
        std::fs::create_dir(&project_path).unwrap();
        
        let analysis = ProjectAnalysis {
            project_path: project_path.clone(),
            project_type: ProjectType::Unknown,
            rust_info: None,
            node_info: None,
            node_projects: vec![],
            file_count: 1,
            total_lines: 10,
            primary_language: "".to_string(),
            languages: HashMap::new(),
            recommendations: vec![],
        };
        
        let name = extract_project_name(&project_path, &analysis).unwrap();
        assert_eq!(name, "my-project");
    }

    #[test]
    fn test_detect_framework_express() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        
        let node_info = NodeProjectInfo {
            package_json_path: path.join("package.json"),
            package_manager: PackageManager::Npm,
            package_name: Some("my-express-app".to_string()),
            version: Some("1.0.0".to_string()),
            dependencies: vec!["express".to_string()],
            dev_dependencies: vec![],
            scripts: HashMap::new(),
            main_entry: Some("index.js".to_string()),
            has_typescript: false,
        };
        
        let analysis = ProjectAnalysis {
            project_path: path.to_path_buf(),
            project_type: ProjectType::NodeApplication,
            rust_info: None,
            node_info: Some(node_info.clone()),
            node_projects: vec![node_info],
            file_count: 5,
            total_lines: 100,
            primary_language: "javascript".to_string(),
            languages: HashMap::new(),
            recommendations: vec![],
        };
        
        let framework = detect_framework(&analysis);
        assert_eq!(framework, Some("express".to_string()));
    }

    #[test]
    fn test_detect_framework_react() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        
        let node_info = NodeProjectInfo {
            package_json_path: path.join("package.json"),
            package_manager: PackageManager::Npm,
            package_name: Some("my-react-app".to_string()),
            version: Some("1.0.0".to_string()),
            dependencies: vec!["react".to_string()],
            dev_dependencies: vec![],
            scripts: HashMap::new(),
            main_entry: Some("index.js".to_string()),
            has_typescript: false,
        };
        
        let analysis = ProjectAnalysis {
            project_path: path.to_path_buf(),
            project_type: ProjectType::NodeApplication,
            rust_info: None,
            node_info: Some(node_info.clone()),
            node_projects: vec![node_info],
            file_count: 5,
            total_lines: 100,
            primary_language: "javascript".to_string(),
            languages: HashMap::new(),
            recommendations: vec![],
        };
        
        let framework = detect_framework(&analysis);
        assert_eq!(framework, Some("react".to_string()));
    }
}
