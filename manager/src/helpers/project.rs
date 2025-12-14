use crate::database::Database;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};
use walkdir::{DirEntry, WalkDir};

/// Types of projects that can be discovered
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectType {
    NodeJs { manager: PackageManager },
    Rust,
    Python { tool: PythonTool },
    Go,
    Java { build_tool: JavaBuildTool },
    Mixed(Vec<ProjectType>),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PythonTool {
    Pip,
    Poetry,
    Pipenv,
    Conda,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JavaBuildTool {
    Maven,
    Gradle,
}

/// A discovered project from the filesystem
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveredProject {
    pub name: String,
    pub path: PathBuf,
    pub project_type: ProjectType,
    pub status: String,
}

/// Error types for project scanning
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Scan a directory for projects and add them to the database
pub async fn scan_filesystem_for_projects(
    scan_path: &Path,
    database: &Database,
) -> Result<Vec<DiscoveredProject>, ScanError> {
    info!("Starting filesystem scan for projects in: {}", scan_path.display());
    
    // Verify the scan path exists and is a directory
    if !scan_path.exists() {
        return Err(ScanError::PathNotFound(scan_path.to_path_buf()));
    }
    
    if !scan_path.is_dir() {
        return Err(ScanError::InvalidPath(format!(
            "Path is not a directory: {}",
            scan_path.display()
        )));
    }
    
    let mut discovered_projects = Vec::new();
    
    // Walk through the directory looking for projects
    let walkdir = WalkDir::new(scan_path)
        .max_depth(3) // Limit depth to avoid scanning too deep
        .into_iter()
        .filter_entry(|e| !is_hidden_directory(e));
    
    for entry in walkdir {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_dir() {
                    if let Some(project) = detect_project(&entry.path()).await {
                        debug!("Discovered project: {} at {}", project.name, project.path.display());
                        
                        // Add to database if not already present
                        match add_project_to_database(&project, database).await {
                            Ok(_) => {
                                discovered_projects.push(project);
                            }
                            Err(e) => {
                                warn!("Failed to add project to database: {}", e);
                                // Continue with other projects even if one fails
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Error walking directory: {}", e);
                // Continue with other entries
            }
        }
    }
    
    info!("Scan completed. Discovered {} projects", discovered_projects.len());
    Ok(discovered_projects)
}

/// Detect if a directory contains a project and return project info
async fn detect_project(project_path: &Path) -> Option<DiscoveredProject> {
    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    let mut detected_types = Vec::new();
    
    // Check for different project types
    if project_path.join("package.json").exists() {
        detected_types.push(ProjectType::NodeJs { 
            manager: detect_package_manager(project_path).await 
        });
    }
    
    if project_path.join("Cargo.toml").exists() {
        detected_types.push(ProjectType::Rust);
    }
    
    if project_path.join("requirements.txt").exists() 
        || project_path.join("pyproject.toml").exists() 
        || project_path.join("Pipfile").exists() 
        || project_path.join("environment.yml").exists() {
        detected_types.push(ProjectType::Python { 
            tool: detect_python_tool(project_path).await 
        });
    }
    
    if project_path.join("go.mod").exists() {
        detected_types.push(ProjectType::Go);
    }
    
    if project_path.join("pom.xml").exists() {
        detected_types.push(ProjectType::Java { 
            build_tool: JavaBuildTool::Maven 
        });
    }
    
    if project_path.join("build.gradle").exists() || project_path.join("build.gradle.kts").exists() {
        detected_types.push(ProjectType::Java { 
            build_tool: JavaBuildTool::Gradle 
        });
    }
    
    // Determine final project type
    let project_type = match detected_types.len() {
        0 => return None, // No project detected
        1 => detected_types.into_iter().next().unwrap(),
        _ => ProjectType::Mixed(detected_types),
    };
    
    Some(DiscoveredProject {
        name: project_name,
        path: project_path.to_path_buf(),
        project_type,
        status: "discovered".to_string(),
    })
}

/// Detect the package manager for a Node.js project
async fn detect_package_manager(project_path: &Path) -> PackageManager {
    if project_path.join("yarn.lock").exists() {
        PackageManager::Yarn
    } else if project_path.join("pnpm-lock.yaml").exists() {
        PackageManager::Pnpm
    } else if project_path.join("bun.lockb").exists() {
        PackageManager::Bun
    } else {
        PackageManager::Npm
    }
}

/// Detect the Python tool being used
async fn detect_python_tool(project_path: &Path) -> PythonTool {
    if project_path.join("pyproject.toml").exists() {
        // Check if it's Poetry by looking for [tool.poetry]
        if let Ok(content) = fs::read_to_string(project_path.join("pyproject.toml")).await {
            if content.contains("[tool.poetry]") {
                return PythonTool::Poetry;
            }
        }
        PythonTool::Pip
    } else if project_path.join("Pipfile").exists() {
        PythonTool::Pipenv
    } else if project_path.join("environment.yml").exists() {
        PythonTool::Conda
    } else {
        PythonTool::Pip
    }
}

/// Add a discovered project to the database
async fn add_project_to_database(
    project: &DiscoveredProject,
    database: &Database,
) -> Result<(), ScanError> {
    use crate::models::Project;
    use chrono::Utc;
    
    // Check if project already exists
    let existing_projects = database
        .get_all_projects()
        .map_err(|e| ScanError::Database(e.to_string()))?;
    
    let project_path_str = project.path.to_string_lossy();
    
    // Skip if project already exists in database
    for existing_project in existing_projects {
        if existing_project.path == project_path_str {
            debug!("Project already exists in database: {}", project_path_str);
            return Ok(());
        }
    }
    
    // Create new project
    let new_project = Project {
        id: 0, // Will be set by database
        name: project.name.clone(),
        path: project_path_str.to_string(),
        description: Some(format!("Discovered {} project", 
            match &project.project_type {
                ProjectType::NodeJs { .. } => "Node.js",
                ProjectType::Rust => "Rust",
                ProjectType::Python { .. } => "Python",
                ProjectType::Go => "Go",
                ProjectType::Java { .. } => "Java",
                ProjectType::Mixed(_) => "Mixed",
                ProjectType::Unknown => "Unknown",
            }
        )),
        parent_id: None,
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
    };
    
    // Add new project to database
    database
        .create_project(&new_project)
        .map_err(|e| ScanError::Database(e.to_string()))?;
    
    info!("Added project to database: {} at {}", project.name, project_path_str);
    Ok(())
}

/// Check if a directory entry should be ignored (hidden directories)
fn is_hidden_directory(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_str().unwrap_or("");
    // Don't filter out the root directory (depth 0)
    if entry.depth() == 0 {
        return false;
    }
    name.starts_with('.') || name == "node_modules" || name == "target"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_scan_finds_nodejs_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project");
        fs::create_dir(&project_dir).await.unwrap();
        
        // Create package.json
        fs::write(
            project_dir.join("package.json"),
            r#"{"name": "test-project", "version": "1.0.0"}"#,
        )
        .await.unwrap();
        

        
        // Create in-memory database for testing
        let db_path = temp_dir.path().join("test.db");
        let database = Database::new(&db_path).unwrap();
        
        let results = scan_filesystem_for_projects(temp_dir.path(), &database)
            .await
            .unwrap();
        

        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test-project");
        match &results[0].project_type {
            ProjectType::NodeJs { manager } => {
                assert_eq!(manager, &PackageManager::Npm);
            }
            _ => panic!("Expected NodeJs project"),
        }
    }

    #[tokio::test]
    async fn test_scan_finds_rust_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("rust-project");
        fs::create_dir(&project_dir).await.unwrap();
        
        // Create Cargo.toml
        fs::write(
            project_dir.join("Cargo.toml"),
            r#"[package]
name = "rust-project"
version = "0.1.0""#,
        )
        .await.unwrap();
        
        let db_path = temp_dir.path().join("test.db");
        let database = Database::new(&db_path).unwrap();
        
        let results = scan_filesystem_for_projects(temp_dir.path(), &database)
            .await
            .unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-project");
        assert_eq!(results[0].project_type, ProjectType::Rust);
    }

    #[tokio::test]
    async fn test_scan_ignores_hidden_directories() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create hidden directory with project
        let hidden_dir = temp_dir.path().join(".hidden");
        fs::create_dir(&hidden_dir).await.unwrap();
        fs::write(
            hidden_dir.join("package.json"),
            r#"{"name": "hidden-project"}"#,
        )
        .await.unwrap();
        
        // Create node_modules directory
        let node_modules_dir = temp_dir.path().join("node_modules");
        fs::create_dir(&node_modules_dir).await.unwrap();
        fs::write(
            node_modules_dir.join("package.json"),
            r#"{"name": "dependency-project"}"#,
        )
        .await.unwrap();
        
        let db_path = temp_dir.path().join("test.db");
        let database = Database::new(&db_path).unwrap();
        
        let results = scan_filesystem_for_projects(temp_dir.path(), &database)
            .await
            .unwrap();
        
        assert_eq!(results.len(), 0); // Should not find hidden projects
    }

    #[tokio::test]
    async fn test_scan_nonexistent_path() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent");
        let db_path = temp_dir.path().join("test.db");
        let database = Database::new(&db_path).unwrap();
        
        let result = scan_filesystem_for_projects(&nonexistent_path, &database).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ScanError::PathNotFound(path) => {
                assert_eq!(path, nonexistent_path);
            }
            _ => panic!("Expected PathNotFound error"),
        }
    }
}