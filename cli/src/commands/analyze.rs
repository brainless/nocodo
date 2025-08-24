//! Project analysis command implementation

use crate::{cli::OutputFormat, error::CliError};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// Helper to create synthetic project structures for testing
    pub struct TestProjectBuilder {
        temp_dir: TempDir,
    }

    impl TestProjectBuilder {
        pub fn new() -> Self {
            Self {
                temp_dir: TempDir::new().expect("Failed to create temp directory"),
            }
        }

        pub fn path(&self) -> &std::path::Path {
            self.temp_dir.path()
        }

        /// Create a Rust application project
        pub fn create_rust_app(&self, name: &str, dependencies: &[&str]) -> &std::path::Path {
            self.create_cargo_toml(name, "bin", dependencies, &[]);
            self.create_file(
                "src/main.rs",
                r#"fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}"#,
            );
            self.path()
        }

        /// Create a Rust library project
        pub fn create_rust_lib(&self, name: &str, dependencies: &[&str]) -> &std::path::Path {
            self.create_cargo_toml(name, "lib", dependencies, &[]);
            self.create_file(
                "src/lib.rs",
                r#"//! A sample library

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}"#,
            );
            self.path()
        }

        /// Create a Rust workspace
        pub fn create_rust_workspace(&self, members: &[&str]) -> &std::path::Path {
            let workspace_cargo_toml = format!(
                r#"[workspace]
resolver = "2"
members = [
{}
]
exclude = []

[workspace.dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
tokio = {{ version = "1.0", features = ["full"] }}
"#,
                members
                    .iter()
                    .map(|m| format!("    \"{m}\""))
                    .collect::<Vec<_>>()
                    .join(",\n")
            );

            self.create_file("Cargo.toml", &workspace_cargo_toml);

            // Create each member
            for member in members {
                self.create_member(member);
            }

            self.path()
        }

        /// Create a Node.js application with npm
        pub fn create_node_app_npm(
            &self,
            name: &str,
            dependencies: &[&str],
            scripts: &[(&str, &str)],
        ) -> &std::path::Path {
            self.create_package_json(name, "1.0.0", Some("index.js"), dependencies, &[], scripts);
            self.create_file(
                "package-lock.json",
                r#"{
  "name": "test-app",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "requires": true,
  "packages": {}
}"#,
            );
            self.create_file(
                "index.js",
                r#"const express = require('express');
const app = express();

app.get('/', (req, res) => {
    res.send('Hello World!');
});

const port = process.env.PORT || 3000;
app.listen(port, () => {
    console.log(`Server running on port ${port}`);
});
"#,
            );
            self.path()
        }

        fn create_cargo_toml(
            &self,
            name: &str,
            project_type: &str,
            dependencies: &[&str],
            dev_dependencies: &[&str],
        ) {
            let deps = if dependencies.is_empty() {
                String::new()
            } else {
                format!(
                    "\n[dependencies]\n{}",
                    dependencies
                        .iter()
                        .map(|d| format!("{d} = \"1.0\""))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            };

            let dev_deps = if dev_dependencies.is_empty() {
                String::new()
            } else {
                format!(
                    "\n[dev-dependencies]\n{}",
                    dev_dependencies
                        .iter()
                        .map(|d| format!("{d} = \"1.0\""))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            };

            let lib_section = if project_type == "lib" {
                "\n[lib]\nname = \"test_lib\"\n"
            } else {
                ""
            };

            let content = format!(
                r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"{lib_section}{deps}{dev_deps}"#
            );

            self.create_file("Cargo.toml", &content);
        }

        fn create_package_json(
            &self,
            name: &str,
            version: &str,
            main: Option<&str>,
            dependencies: &[&str],
            dev_dependencies: &[&str],
            scripts: &[(&str, &str)],
        ) {
            let mut json = serde_json::json!({
                "name": name,
                "version": version,
            });

            if let Some(main_file) = main {
                json["main"] = serde_json::Value::String(main_file.to_string());
            }

            if !dependencies.is_empty() {
                let deps: HashMap<&str, &str> =
                    dependencies.iter().map(|&d| (d, "^1.0.0")).collect();
                json["dependencies"] = serde_json::to_value(deps).unwrap();
            }

            if !dev_dependencies.is_empty() {
                let dev_deps: HashMap<&str, &str> =
                    dev_dependencies.iter().map(|&d| (d, "^1.0.0")).collect();
                json["devDependencies"] = serde_json::to_value(dev_deps).unwrap();
            }

            if !scripts.is_empty() {
                let script_map: HashMap<&str, &str> = scripts.iter().copied().collect();
                json["scripts"] = serde_json::to_value(script_map).unwrap();
            }

            self.create_file(
                "package.json",
                &serde_json::to_string_pretty(&json).unwrap(),
            );
        }

        fn create_member(&self, name: &str) {
            let member_dir = self.path().join(name);
            std::fs::create_dir_all(member_dir.join("src")).unwrap();

            let cargo_toml = format!(
                r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ workspace = true }}
"#
            );

            std::fs::write(member_dir.join("Cargo.toml"), cargo_toml).unwrap();
            std::fs::write(
                member_dir.join("src/lib.rs"),
                "pub fn hello() { println!(\"Hello from {}!\"); }",
            )
            .unwrap();
        }

        fn create_file(&self, path: &str, content: &str) {
            let file_path = self.path().join(path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(file_path, content).unwrap();
        }
    }

    #[tokio::test]
    async fn test_rust_application_analysis() {
        let builder = TestProjectBuilder::new();
        let project_path = builder.create_rust_app("test-app", &["serde", "tokio", "clap"]);

        let analyzer = ProjectAnalyzer::new();
        let analysis = analyzer.analyze(project_path).await.unwrap();

        assert_eq!(analysis.project_type, ProjectType::RustApplication);
        assert!(analysis.rust_info.is_some());
        assert!(analysis.node_info.is_none());
        assert_eq!(analysis.primary_language, "rust");

        let rust_info = analysis.rust_info.as_ref().unwrap();
        assert_eq!(rust_info.package_name, Some("test-app".to_string()));
        assert!(!rust_info.is_workspace);
        assert!(!rust_info.lib_target);
        assert!(rust_info.bin_targets.contains(&"test-app".to_string()));
        // Dependencies order may vary due to HashMap
        assert_eq!(rust_info.dependencies.len(), 3);
        assert!(rust_info.dependencies.contains(&"serde".to_string()));
        assert!(rust_info.dependencies.contains(&"tokio".to_string()));
        assert!(rust_info.dependencies.contains(&"clap".to_string()));
    }

    #[tokio::test]
    async fn test_rust_library_analysis() {
        let builder = TestProjectBuilder::new();
        let project_path = builder.create_rust_lib("test-lib", &["serde"]);

        let analyzer = ProjectAnalyzer::new();
        let analysis = analyzer.analyze(project_path).await.unwrap();

        assert_eq!(analysis.project_type, ProjectType::RustLibrary);

        let rust_info = analysis.rust_info.as_ref().unwrap();
        assert_eq!(rust_info.package_name, Some("test-lib".to_string()));
        assert!(!rust_info.is_workspace);
        assert!(rust_info.lib_target);
        assert_eq!(rust_info.dependencies.len(), 1);
        assert!(rust_info.dependencies.contains(&"serde".to_string()));
    }

    #[tokio::test]
    async fn test_rust_workspace_analysis() {
        let builder = TestProjectBuilder::new();
        let project_path = builder.create_rust_workspace(&["api", "web", "shared"]);

        let analyzer = ProjectAnalyzer::new();
        let analysis = analyzer.analyze(project_path).await.unwrap();

        assert_eq!(analysis.project_type, ProjectType::RustWorkspace);

        let rust_info = analysis.rust_info.as_ref().unwrap();
        assert!(rust_info.is_workspace);
        assert_eq!(rust_info.workspace_members, vec!["api", "web", "shared"]);
        assert!(rust_info.package_name.is_none()); // Workspace root has no package
    }

    #[tokio::test]
    async fn test_node_app_npm_analysis() {
        let builder = TestProjectBuilder::new();
        let project_path = builder.create_node_app_npm(
            "test-node-app",
            &["express", "dotenv"],
            &[("start", "node index.js"), ("test", "npm test")],
        );

        let analyzer = ProjectAnalyzer::new();
        let analysis = analyzer.analyze(project_path).await.unwrap();

        assert_eq!(analysis.project_type, ProjectType::NodeApplication);
        assert!(analysis.node_info.is_some());
        assert!(analysis.rust_info.is_none());

        let node_info = analysis.node_info.as_ref().unwrap();
        assert_eq!(node_info.package_name, Some("test-node-app".to_string()));
        assert_eq!(node_info.package_manager, PackageManager::Npm);
        assert_eq!(node_info.main_entry, Some("index.js".to_string()));
        // Dependencies order may vary due to HashMap
        assert_eq!(node_info.dependencies.len(), 2);
        assert!(node_info.dependencies.contains(&"express".to_string()));
        assert!(node_info.dependencies.contains(&"dotenv".to_string()));
        assert_eq!(
            node_info.scripts.get("start"),
            Some(&"node index.js".to_string())
        );
        assert!(!node_info.has_typescript);
    }

    #[tokio::test]
    async fn test_unknown_project_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // Create a directory with no recognizable project files
        std::fs::write(project_path.join("README.md"), "# Test Project").unwrap();
        std::fs::write(project_path.join("random.txt"), "Some content").unwrap();

        let analyzer = ProjectAnalyzer::new();
        let analysis = analyzer.analyze(project_path).await.unwrap();

        assert_eq!(analysis.project_type, ProjectType::Unknown);
        assert!(analysis.rust_info.is_none());
        assert!(analysis.node_info.is_none());
        assert!(analysis.primary_language.is_empty());
    }

    #[tokio::test]
    async fn test_polyglot_rust_workspace_with_node_projects() {
        let builder = TestProjectBuilder::new();
        let project_path = builder.create_rust_workspace(&["api", "core"]);

        // Add Node.js projects in subdirectories
        builder.create_file(
            "web/package.json",
            r#"{
  "name": "web-frontend",
  "version": "1.0.0",
  "main": "index.js",
  "dependencies": {
    "express": "^4.18.0",
    "react": "^18.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  },
  "scripts": {
    "start": "node index.js",
    "build": "tsc",
    "test": "jest"
  }
}"#,
        );

        builder.create_file(
            "mobile/package.json",
            r#"{
  "name": "mobile-app",
  "version": "0.2.0",
  "dependencies": {
    "react-native": "^0.72.0"
  },
  "scripts": {
    "start": "react-native start"
  }
}"#,
        );

        // Add lock files
        builder.create_file("web/package-lock.json", "{}");
        builder.create_file("mobile/yarn.lock", "");

        let analyzer = ProjectAnalyzer::new();
        let analysis = analyzer.analyze(project_path).await.unwrap();

        // Should be detected as Rust workspace (primary)
        assert_eq!(analysis.project_type, ProjectType::RustWorkspace);
        assert_eq!(analysis.primary_language, "rust");

        // Should have Rust info
        assert!(analysis.rust_info.is_some());
        let rust_info = analysis.rust_info.as_ref().unwrap();
        assert!(rust_info.is_workspace);
        assert_eq!(rust_info.workspace_members, vec!["api", "core"]);

        // Should detect multiple Node.js projects
        assert_eq!(analysis.node_projects.len(), 2);

        // Find web project
        let web_project = analysis
            .node_projects
            .iter()
            .find(|p| p.package_name.as_deref() == Some("web-frontend"))
            .expect("Should find web project");
        assert_eq!(web_project.version, Some("1.0.0".to_string()));
        assert_eq!(web_project.package_manager, PackageManager::Npm);
        assert_eq!(web_project.dependencies.len(), 2);
        assert!(web_project.dependencies.contains(&"express".to_string()));
        assert!(web_project.dependencies.contains(&"react".to_string()));
        assert!(web_project.has_typescript);
        assert_eq!(web_project.scripts.len(), 3);

        // Find mobile project
        let mobile_project = analysis
            .node_projects
            .iter()
            .find(|p| p.package_name.as_deref() == Some("mobile-app"))
            .expect("Should find mobile project");
        assert_eq!(mobile_project.version, Some("0.2.0".to_string()));
        assert_eq!(mobile_project.package_manager, PackageManager::Yarn);
        assert_eq!(mobile_project.dependencies.len(), 1);
        assert!(mobile_project
            .dependencies
            .contains(&"react-native".to_string()));
        assert!(!mobile_project.has_typescript);
        assert_eq!(mobile_project.scripts.len(), 1);

        // Should have backward compatibility
        assert!(analysis.node_info.is_some());
        assert_eq!(
            analysis.node_info.as_ref().unwrap().package_name,
            analysis.node_projects.first().unwrap().package_name
        );
    }
}

/// Project type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectType {
    RustApplication,
    RustLibrary,
    RustWorkspace,
    NodeApplication,
    NodeLibrary,
    Unknown,
}

/// Package manager for Node.js projects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Unknown,
}

/// Rust project information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustProjectInfo {
    pub cargo_toml_path: PathBuf,
    pub is_workspace: bool,
    pub workspace_members: Vec<String>,
    pub package_name: Option<String>,
    pub dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
    pub bin_targets: Vec<String>,
    pub lib_target: bool,
}

/// Node.js project information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProjectInfo {
    pub package_json_path: PathBuf,
    pub package_manager: PackageManager,
    pub package_name: Option<String>,
    pub version: Option<String>,
    pub dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
    pub scripts: HashMap<String, String>,
    pub main_entry: Option<String>,
    pub has_typescript: bool,
}

/// Complete project analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAnalysis {
    pub project_path: PathBuf,
    pub project_type: ProjectType,
    pub rust_info: Option<RustProjectInfo>,
    pub node_info: Option<NodeProjectInfo>, // For backward compatibility when there's only one
    pub node_projects: Vec<NodeProjectInfo>, // Support for multiple Node.js projects
    pub file_count: usize,
    pub total_lines: usize,
    pub primary_language: String,
    pub languages: HashMap<String, usize>, // language -> line count
    pub recommendations: Vec<String>,
}

/// Analyze a project and provide recommendations
pub async fn analyze_project(
    path: &Option<PathBuf>,
    format: &Option<OutputFormat>,
) -> Result<(), CliError> {
    let target_path = path
        .as_ref()
        .map(|p| p.as_path())
        .unwrap_or_else(|| std::path::Path::new("."));

    let canonical_path = target_path.canonicalize().map_err(|e| {
        CliError::Analysis(format!("Failed to resolve path {target_path:?}: {e}"))
    })?;

    let analyzer = ProjectAnalyzer::new();
    let analysis = analyzer.analyze(&canonical_path).await?;

    let output_format = format.as_ref().unwrap_or(&OutputFormat::Text);

    match output_format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&analysis)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&analysis)?);
        }
        OutputFormat::Text => {
            print_text_analysis(&analysis);
        }
    }

    Ok(())
}

fn print_text_analysis(analysis: &ProjectAnalysis) {
    println!("\n🔍 Project Analysis Report");
    println!("{}", "═".repeat(50));
    println!("📁 Path: {}", analysis.project_path.display());
    println!("🏷️  Type: {:?}", analysis.project_type);
    println!(
        "📄 Files: {} ({} total lines)",
        analysis.file_count, analysis.total_lines
    );
    println!("🌐 Primary Language: {}", analysis.primary_language);

    if !analysis.languages.is_empty() {
        println!("\n📊 Languages Detected:");
        for (lang, count) in &analysis.languages {
            println!("   {lang}: {count} lines");
        }
    }

    // Rust-specific information
    if let Some(rust_info) = &analysis.rust_info {
        println!("\n🦀 Rust Project Information:");
        if let Some(name) = &rust_info.package_name {
            println!("   Package: {name}");
        }
        println!(
            "   Workspace: {}",
            if rust_info.is_workspace { "Yes" } else { "No" }
        );

        if rust_info.is_workspace && !rust_info.workspace_members.is_empty() {
            println!("   Members: {}", rust_info.workspace_members.join(", "));
        }

        if !rust_info.dependencies.is_empty() {
            println!("   Dependencies: {}", rust_info.dependencies.len());
        }

        if rust_info.lib_target {
            println!("   Library: Yes");
        }

        if !rust_info.bin_targets.is_empty() {
            println!("   Binaries: {}", rust_info.bin_targets.join(", "));
        }
    }

    // Node.js-specific information
    if !analysis.node_projects.is_empty() {
        if analysis.node_projects.len() == 1 {
            // Single Node.js project - use the compact format
            if let Some(node_info) = analysis.node_projects.first() {
                println!("\n🟢 Node.js Project Information:");
                if let Some(name) = &node_info.package_name {
                    println!("   Package: {name}");
                }
                if let Some(version) = &node_info.version {
                    println!("   Version: {version}");
                }
                println!("   Package Manager: {:?}", node_info.package_manager);

                if !node_info.dependencies.is_empty() {
                    println!("   Dependencies: {}", node_info.dependencies.len());
                }

                if node_info.has_typescript {
                    println!("   TypeScript: Yes");
                }

                if !node_info.scripts.is_empty() {
                    println!("   Scripts: {}", node_info.scripts.len());
                }
            }
        } else {
            // Multiple Node.js projects - show all of them
            println!("\n🟢 Node.js Projects ({}):", analysis.node_projects.len());
            for (i, node_info) in analysis.node_projects.iter().enumerate() {
                let project_path = node_info
                    .package_json_path
                    .parent()
                    .and_then(|p| p.strip_prefix(&analysis.project_path).ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string());

                println!(
                    "   {}. {} ({})",
                    i + 1,
                    node_info.package_name.as_deref().unwrap_or("<unnamed>"),
                    project_path
                );

                if let Some(version) = &node_info.version {
                    println!("      Version: {version}");
                }

                println!("      Package Manager: {:?}", node_info.package_manager);

                if !node_info.dependencies.is_empty() {
                    println!("      Dependencies: {}", node_info.dependencies.len());
                }

                if node_info.has_typescript {
                    println!("      TypeScript: Yes");
                }

                if !node_info.scripts.is_empty() {
                    println!("      Scripts: {}", node_info.scripts.len());
                }
            }
        }
    }

    // Recommendations
    if !analysis.recommendations.is_empty() {
        println!("\n💡 Recommendations:");
        for rec in &analysis.recommendations {
            println!("   • {rec}");
        }
    }

    println!();
}

/// Project analyzer implementation
pub struct ProjectAnalyzer {
    // Future: Add configuration for ignoring files, etc.
}

impl ProjectAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn analyze(&self, path: &Path) -> Result<ProjectAnalysis, CliError> {
        if !path.exists() {
            return Err(CliError::Analysis(format!(
                "Path does not exist: {path:?}"
            )));
        }

        if !path.is_dir() {
            return Err(CliError::Analysis(format!(
                "Path is not a directory: {path:?}"
            )));
        }

        let mut analysis = ProjectAnalysis {
            project_path: path.to_path_buf(),
            project_type: ProjectType::Unknown,
            rust_info: None,
            node_info: None,
            node_projects: Vec::new(),
            file_count: 0,
            total_lines: 0,
            primary_language: String::new(),
            languages: HashMap::new(),
            recommendations: Vec::new(),
        };

        // Detect project type and analyze accordingly
        if let Some(rust_info) = self.analyze_rust_project(path).await? {
            analysis.project_type = if rust_info.is_workspace {
                ProjectType::RustWorkspace
            } else if rust_info.lib_target {
                ProjectType::RustLibrary
            } else {
                ProjectType::RustApplication
            };
            analysis.rust_info = Some(rust_info);
            analysis.primary_language = "rust".to_string();
        }

        // Scan for all Node.js projects in the directory tree
        let node_projects = self.find_all_node_projects(path).await?;
        if !node_projects.is_empty() {
            // Set the primary node_info to the first one found (for backward compatibility)
            analysis.node_info = node_projects.first().cloned();
            analysis.node_projects = node_projects;

            // If we already detected Rust, this is a polyglot project
            if analysis.project_type == ProjectType::Unknown {
                if let Some(first_node) = analysis.node_info.as_ref() {
                    analysis.project_type = if first_node.main_entry.is_some() {
                        ProjectType::NodeApplication
                    } else {
                        ProjectType::NodeLibrary
                    };
                    analysis.primary_language = "javascript".to_string();
                }
            }
        }

        // Count files and lines
        self.count_files_and_lines(path, &mut analysis).await?;

        // Generate recommendations
        self.generate_recommendations(&mut analysis);

        Ok(analysis)
    }

    async fn analyze_rust_project(&self, path: &Path) -> Result<Option<RustProjectInfo>, CliError> {
        let cargo_toml_path = path.join("Cargo.toml");

        if !cargo_toml_path.exists() {
            return Ok(None);
        }

        let cargo_content = fs::read_to_string(&cargo_toml_path)
            .map_err(|e| CliError::Analysis(format!("Failed to read Cargo.toml: {e}")))?;

        let cargo_toml: toml::Value = cargo_content
            .parse()
            .map_err(|e| CliError::Analysis(format!("Failed to parse Cargo.toml: {e}")))?;

        let mut rust_info = RustProjectInfo {
            cargo_toml_path,
            is_workspace: false,
            workspace_members: Vec::new(),
            package_name: None,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            bin_targets: Vec::new(),
            lib_target: false,
        };

        // Check if it's a workspace
        if let Some(workspace) = cargo_toml.get("workspace") {
            rust_info.is_workspace = true;

            if let Some(members) = workspace.get("members") {
                if let Some(members_array) = members.as_array() {
                    rust_info.workspace_members = members_array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }

            // For workspaces, analyze member packages for binaries and libraries
            for member in &rust_info.workspace_members {
                if let Ok(member_info) = self.analyze_workspace_member(path, member).await {
                    rust_info.bin_targets.extend(member_info.bin_targets);
                    rust_info.dependencies.extend(member_info.dependencies);
                    rust_info
                        .dev_dependencies
                        .extend(member_info.dev_dependencies);
                    if member_info.lib_target {
                        rust_info.lib_target = true;
                    }
                }
            }

            // For workspaces, we've already analyzed members, so return early
            return Ok(Some(rust_info));
        }

        // Extract package information
        if let Some(package) = cargo_toml.get("package") {
            if let Some(name) = package.get("name") {
                rust_info.package_name = name.as_str().map(|s| s.to_string());
            }
        }

        // Extract dependencies
        if let Some(deps) = cargo_toml.get("dependencies") {
            if let Some(deps_table) = deps.as_table() {
                rust_info.dependencies = deps_table.keys().cloned().collect();
            }
        }

        // Extract dev dependencies
        if let Some(dev_deps) = cargo_toml.get("dev-dependencies") {
            if let Some(dev_deps_table) = dev_deps.as_table() {
                rust_info.dev_dependencies = dev_deps_table.keys().cloned().collect();
            }
        }

        // Check for binary targets
        if let Some(bins) = cargo_toml.get("bin") {
            if let Some(bins_array) = bins.as_array() {
                for bin in bins_array {
                    if let Some(name) = bin.get("name").and_then(|v| v.as_str()) {
                        rust_info.bin_targets.push(name.to_string());
                    }
                }
            }
        }

        // Check for library target
        rust_info.lib_target =
            path.join("src").join("lib.rs").exists() || cargo_toml.get("lib").is_some();

        // If no explicit binary targets but has main.rs, add it
        if rust_info.bin_targets.is_empty() && path.join("src").join("main.rs").exists() {
            if let Some(name) = &rust_info.package_name {
                rust_info.bin_targets.push(name.clone());
            } else {
                rust_info.bin_targets.push("main".to_string());
            }
        }

        Ok(Some(rust_info))
    }


    async fn find_all_node_projects(
        &self,
        root_path: &Path,
    ) -> Result<Vec<NodeProjectInfo>, CliError> {
        let mut node_projects = Vec::new();
        let walker = WalkBuilder::new(root_path)
            .max_depth(Some(3)) // Limit depth to avoid deep recursion
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            let entry =
                entry.map_err(|e| CliError::Analysis(format!("Error walking directory: {e}")))?;

            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let file_path = entry.path();

            // Skip if not package.json
            if file_path.file_name().and_then(|n| n.to_str()) != Some("package.json") {
                continue;
            }

            // Skip if inside node_modules or other build directories
            let path_string = file_path.to_string_lossy();
            if path_string.contains("/node_modules/")
                || path_string.contains("/dist/")
                || path_string.contains("/build/")
                || path_string.contains("/.git/")
            {
                continue;
            }

            // Get the directory containing the package.json
            if let Some(project_dir) = file_path.parent() {
                if let Ok(Some(node_info)) = self.analyze_node_project_at_path(project_dir).await {
                    node_projects.push(node_info);
                }
            }
        }

        Ok(node_projects)
    }

    async fn analyze_node_project_at_path(
        &self,
        path: &Path,
    ) -> Result<Option<NodeProjectInfo>, CliError> {
        let package_json_path = path.join("package.json");

        if !package_json_path.exists() {
            return Ok(None);
        }

        let package_content = fs::read_to_string(&package_json_path)
            .map_err(|e| CliError::Analysis(format!("Failed to read package.json: {e}")))?;

        let package_json: serde_json::Value = serde_json::from_str(&package_content)
            .map_err(|e| CliError::Analysis(format!("Failed to parse package.json: {e}")))?;

        let mut node_info = NodeProjectInfo {
            package_json_path,
            package_manager: PackageManager::Unknown,
            package_name: None,
            version: None,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            scripts: HashMap::new(),
            main_entry: None,
            has_typescript: false,
        };

        // Extract package name and version
        if let Some(name) = package_json.get("name").and_then(|v| v.as_str()) {
            node_info.package_name = Some(name.to_string());
        }

        if let Some(version) = package_json.get("version").and_then(|v| v.as_str()) {
            node_info.version = Some(version.to_string());
        }

        // Extract main entry point
        if let Some(main) = package_json.get("main").and_then(|v| v.as_str()) {
            node_info.main_entry = Some(main.to_string());
        }

        // Extract dependencies
        if let Some(deps) = package_json.get("dependencies").and_then(|v| v.as_object()) {
            node_info.dependencies = deps.keys().cloned().collect();
        }

        // Extract dev dependencies
        if let Some(dev_deps) = package_json
            .get("devDependencies")
            .and_then(|v| v.as_object())
        {
            node_info.dev_dependencies = dev_deps.keys().cloned().collect();
        }

        // Extract scripts
        if let Some(scripts) = package_json.get("scripts").and_then(|v| v.as_object()) {
            for (key, value) in scripts {
                if let Some(script_value) = value.as_str() {
                    node_info
                        .scripts
                        .insert(key.clone(), script_value.to_string());
                }
            }
        }

        // Detect package manager
        node_info.package_manager = self.detect_package_manager(path);

        // Check for TypeScript
        node_info.has_typescript = node_info.dependencies.contains(&"typescript".to_string())
            || node_info
                .dev_dependencies
                .contains(&"typescript".to_string())
            || path.join("tsconfig.json").exists();

        Ok(Some(node_info))
    }

    fn detect_package_manager(&self, path: &Path) -> PackageManager {
        if path.join("pnpm-lock.yaml").exists() {
            PackageManager::Pnpm
        } else if path.join("yarn.lock").exists() {
            PackageManager::Yarn
        } else if path.join("package-lock.json").exists() {
            PackageManager::Npm
        } else {
            PackageManager::Unknown
        }
    }

    async fn analyze_workspace_member(
        &self,
        workspace_path: &Path,
        member_name: &str,
    ) -> Result<RustProjectInfo, CliError> {
        let member_path = workspace_path.join(member_name);
        let member_cargo_toml = member_path.join("Cargo.toml");

        if !member_cargo_toml.exists() {
            return Err(CliError::Analysis(format!(
                "Member {member_name} has no Cargo.toml"
            )));
        }

        let cargo_content = fs::read_to_string(&member_cargo_toml).map_err(|e| {
            CliError::Analysis(format!("Failed to read {member_name}/Cargo.toml: {e}"))
        })?;

        let cargo_toml: toml::Value = cargo_content.parse().map_err(|e| {
            CliError::Analysis(format!("Failed to parse {member_name}/Cargo.toml: {e}"))
        })?;

        let mut member_info = RustProjectInfo {
            cargo_toml_path: member_cargo_toml,
            is_workspace: false, // Members are not workspaces themselves
            workspace_members: Vec::new(),
            package_name: None,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            bin_targets: Vec::new(),
            lib_target: false,
        };

        // Extract package information
        if let Some(package) = cargo_toml.get("package") {
            if let Some(name) = package.get("name") {
                member_info.package_name = name.as_str().map(|s| s.to_string());
            }
        }

        // Extract dependencies
        if let Some(deps) = cargo_toml.get("dependencies") {
            if let Some(deps_table) = deps.as_table() {
                member_info.dependencies = deps_table.keys().cloned().collect();
            }
        }

        // Extract dev dependencies
        if let Some(dev_deps) = cargo_toml.get("dev-dependencies") {
            if let Some(dev_deps_table) = dev_deps.as_table() {
                member_info.dev_dependencies = dev_deps_table.keys().cloned().collect();
            }
        }

        // Check for binary targets
        if let Some(bins) = cargo_toml.get("bin") {
            if let Some(bins_array) = bins.as_array() {
                for bin in bins_array {
                    if let Some(name) = bin.get("name").and_then(|v| v.as_str()) {
                        member_info.bin_targets.push(name.to_string());
                    }
                }
            }
        }

        // Check for library target
        member_info.lib_target =
            member_path.join("src").join("lib.rs").exists() || cargo_toml.get("lib").is_some();

        // If no explicit binary targets but has main.rs, add it
        if member_info.bin_targets.is_empty() && member_path.join("src").join("main.rs").exists() {
            if let Some(name) = &member_info.package_name {
                member_info.bin_targets.push(name.clone());
            } else {
                member_info.bin_targets.push("main".to_string());
            }
        }

        Ok(member_info)
    }

    async fn count_files_and_lines(
        &self,
        path: &Path,
        analysis: &mut ProjectAnalysis,
    ) -> Result<(), CliError> {
        let walker = WalkBuilder::new(path)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            let entry =
                entry.map_err(|e| CliError::Analysis(format!("Error walking directory: {e}")))?;

            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let file_path = entry.path();

            // Skip hidden files and common build/cache directories
            if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with('.') {
                    continue;
                }
            }

            if let Some(parent) = file_path.parent() {
                if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                    if matches!(
                        parent_name,
                        "target" | "node_modules" | "dist" | "build" | ".git"
                    ) {
                        continue;
                    }
                }
            }

            analysis.file_count += 1;

            // Count lines and detect language
            if let Ok(content) = fs::read_to_string(file_path) {
                let line_count = content.lines().count();
                analysis.total_lines += line_count;

                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                    let language = match ext {
                        "rs" => "rust",
                        "js" | "mjs" | "cjs" => "javascript",
                        "ts" | "mts" | "cts" => "typescript",
                        "json" => "json",
                        "toml" => "toml",
                        "yaml" | "yml" => "yaml",
                        "md" => "markdown",
                        "html" => "html",
                        "css" => "css",
                        "py" => "python",
                        "go" => "go",
                        "java" => "java",
                        "c" => "c",
                        "cpp" | "cc" | "cxx" => "cpp",
                        "h" | "hpp" => "header",
                        _ => "other",
                    };

                    *analysis.languages.entry(language.to_string()).or_insert(0) += line_count;
                }
            }
        }

        Ok(())
    }

    fn generate_recommendations(&self, analysis: &mut ProjectAnalysis) {
        // Rust-specific recommendations
        if let Some(rust_info) = &analysis.rust_info {
            if rust_info.dependencies.is_empty() && !rust_info.is_workspace {
                analysis
                    .recommendations
                    .push("Consider adding dependencies to enhance functionality".to_string());
            }

            if !rust_info.lib_target && rust_info.bin_targets.is_empty() {
                analysis.recommendations.push("No library or binary targets found - consider adding src/main.rs or src/lib.rs".to_string());
            }

            if rust_info.is_workspace && rust_info.workspace_members.is_empty() {
                analysis.recommendations.push("Workspace detected but no members found - check workspace.members in Cargo.toml".to_string());
            }
        }

        // Node.js-specific recommendations
        if let Some(node_info) = &analysis.node_info {
            if node_info.package_manager == PackageManager::Unknown {
                analysis.recommendations.push("No lock file detected - consider using npm, yarn, or pnpm for dependency management".to_string());
            }

            if node_info.scripts.is_empty() {
                analysis.recommendations.push(
                    "No npm scripts found - consider adding build, test, or start scripts"
                        .to_string(),
                );
            }

            if node_info.has_typescript
                && !node_info
                    .dev_dependencies
                    .contains(&"@types/node".to_string())
            {
                analysis.recommendations.push("TypeScript detected but @types/node not found - consider adding it for better type support".to_string());
            }
        }

        // General recommendations
        if analysis.file_count == 0 {
            analysis.recommendations.push(
                "No source files found - ensure you're in the correct project directory"
                    .to_string(),
            );
        } else if analysis.file_count > 1000 {
            analysis.recommendations.push(
                "Large project detected - consider organizing code into modules or workspaces"
                    .to_string(),
            );
        }

        if analysis.total_lines > 10000 {
            analysis.recommendations.push(
                "Large codebase detected - consider code documentation and testing strategies"
                    .to_string(),
            );
        }
    }
}
