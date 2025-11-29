use crate::error::AppResult;
use crate::models::ProjectCommand;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info};
use uuid::Uuid;

/// Types of projects supported
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    NodeJs { manager: PackageManager },
    Rust,
    Python { tool: PythonTool },
    Go,
    Java { build_tool: JavaBuildTool },
    Mixed(Vec<ProjectType>),
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PythonTool {
    Pip,
    Poetry,
    Pipenv,
    Conda,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JavaBuildTool {
    Maven,
    Gradle,
}

/// A suggested command discovered from the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedCommand {
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    pub environment: Option<HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub os_filter: Option<Vec<String>>,
}

impl SuggestedCommand {
    pub fn to_project_command(&self, project_id: i64) -> ProjectCommand {
        let now = chrono::Utc::now().timestamp();
        ProjectCommand {
            id: Uuid::new_v4().to_string(),
            project_id,
            name: self.name.clone(),
            description: self.description.clone(),
            command: self.command.clone(),
            shell: self.shell.clone(),
            working_directory: self.working_directory.clone(),
            environment: self.environment.clone(),
            timeout_seconds: self.timeout_seconds,
            os_filter: self.os_filter.clone(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Response from command discovery
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoverCommandsResponse {
    pub commands: Vec<SuggestedCommand>,
    pub project_types: Vec<String>,
    pub reasoning: Option<String>,
}

/// Main command discovery engine
pub struct CommandDiscovery {
    project_path: PathBuf,
    #[allow(dead_code)]
    project_id: i64,
}

impl CommandDiscovery {
    pub fn new(project_path: PathBuf, project_id: i64) -> Self {
        Self {
            project_path,
            project_id,
        }
    }

    /// Detect project type and tech stack
    pub async fn detect_project_type(&self) -> AppResult<ProjectType> {
        debug!("Detecting project type for: {:?}", self.project_path);

        let mut detected_types = Vec::new();

        // Check for Node.js
        if self.has_file("package.json").await? {
            let manager = self.detect_package_manager().await?;
            detected_types.push(ProjectType::NodeJs { manager });
        }

        // Check for Rust
        if self.has_file("Cargo.toml").await? {
            detected_types.push(ProjectType::Rust);
        }

        // Check for Python
        if let Some(tool) = self.detect_python_tool().await? {
            detected_types.push(ProjectType::Python { tool });
        }

        // Check for Go
        if self.has_file("go.mod").await? {
            detected_types.push(ProjectType::Go);
        }

        // Check for Java
        if let Some(build_tool) = self.detect_java_build_tool().await? {
            detected_types.push(ProjectType::Java { build_tool });
        }

        let project_type = match detected_types.len() {
            0 => ProjectType::Unknown,
            1 => detected_types.into_iter().next().unwrap(),
            _ => ProjectType::Mixed(detected_types),
        };

        info!("Detected project type: {:?}", project_type);
        Ok(project_type)
    }

    /// Discover all commands from various sources
    pub async fn discover_all(&self) -> AppResult<DiscoverCommandsResponse> {
        let project_type = self.detect_project_type().await?;
        let mut all_commands = Vec::new();
        let mut project_types = Vec::new();

        match &project_type {
            ProjectType::NodeJs { manager } => {
                project_types.push(format!("Node.js ({:?})", manager));
                all_commands.extend(self.discover_npm_commands().await?);
            }
            ProjectType::Rust => {
                project_types.push("Rust".to_string());
                all_commands.extend(self.discover_cargo_commands().await?);
            }
            ProjectType::Python { tool } => {
                project_types.push(format!("Python ({:?})", tool));
                all_commands.extend(self.discover_python_commands().await?);
            }
            ProjectType::Go => {
                project_types.push("Go".to_string());
                all_commands.extend(self.discover_go_commands().await?);
            }
            ProjectType::Java { build_tool } => {
                project_types.push(format!("Java ({:?})", build_tool));
                all_commands.extend(self.discover_java_commands().await?);
            }
            ProjectType::Mixed(types) => {
                for t in types {
                    match t {
                        ProjectType::NodeJs { manager } => {
                            project_types.push(format!("Node.js ({:?})", manager));
                            all_commands.extend(self.discover_npm_commands().await?);
                        }
                        ProjectType::Rust => {
                            project_types.push("Rust".to_string());
                            all_commands.extend(self.discover_cargo_commands().await?);
                        }
                        ProjectType::Python { tool } => {
                            project_types.push(format!("Python ({:?})", tool));
                            all_commands.extend(self.discover_python_commands().await?);
                        }
                        ProjectType::Go => {
                            project_types.push("Go".to_string());
                            all_commands.extend(self.discover_go_commands().await?);
                        }
                        ProjectType::Java { build_tool } => {
                            project_types.push(format!("Java ({:?})", build_tool));
                            all_commands.extend(self.discover_java_commands().await?);
                        }
                        _ => {}
                    }
                }
            }
            ProjectType::Unknown => {
                project_types.push("Unknown".to_string());
            }
        }

        // Try Makefile discovery regardless of project type
        if let Ok(make_commands) = self.discover_make_commands().await {
            all_commands.extend(make_commands);
        }

        let num_commands = all_commands.len();
        let num_types = project_types.len();

        Ok(DiscoverCommandsResponse {
            commands: all_commands,
            project_types,
            reasoning: Some(format!(
                "Discovered {} commands from {} project type(s)",
                num_commands,
                num_types
            )),
        })
    }

    /// Discover commands from package.json scripts
    pub async fn discover_npm_commands(&self) -> AppResult<Vec<SuggestedCommand>> {
        let package_json_path = self.project_path.join("package.json");
        if !package_json_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&package_json_path).await?;
        let package: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut commands = Vec::new();

        // Get package manager command prefix
        let manager = self.detect_package_manager().await?;
        let pm_cmd = match manager {
            PackageManager::Npm => "npm",
            PackageManager::Yarn => "yarn",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Bun => "bun",
        };

        // Add install command
        commands.push(SuggestedCommand {
            name: "install".to_string(),
            description: Some("Install project dependencies".to_string()),
            command: format!("{} install", pm_cmd),
            shell: Some("bash".to_string()),
            working_directory: None,
            environment: None,
            timeout_seconds: Some(300),
            os_filter: None,
        });

        // Parse scripts section
        if let Some(scripts) = package["scripts"].as_object() {
            for (script_name, script_value) in scripts {
                if let Some(script_cmd) = script_value.as_str() {
                    let description = Self::infer_script_description(script_name, script_cmd);

                    commands.push(SuggestedCommand {
                        name: script_name.clone(),
                        description: Some(description),
                        command: format!("{} run {}", pm_cmd, script_name),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: Self::infer_environment(script_name),
                        timeout_seconds: Some(120),
                        os_filter: None,
                    });
                }
            }
        }

        info!("Discovered {} npm commands", commands.len());
        Ok(commands)
    }

    /// Discover Rust/Cargo commands
    pub async fn discover_cargo_commands(&self) -> AppResult<Vec<SuggestedCommand>> {
        if !self.has_file("Cargo.toml").await? {
            return Ok(Vec::new());
        }

        let mut commands = vec![
            SuggestedCommand {
                name: "cargo-build".to_string(),
                description: Some("Build the project".to_string()),
                command: "cargo build".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(600),
                os_filter: None,
            },
            SuggestedCommand {
                name: "cargo-build-release".to_string(),
                description: Some("Build the project in release mode".to_string()),
                command: "cargo build --release".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(900),
                os_filter: None,
            },
            SuggestedCommand {
                name: "cargo-test".to_string(),
                description: Some("Run tests".to_string()),
                command: "cargo test".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(600),
                os_filter: None,
            },
            SuggestedCommand {
                name: "cargo-check".to_string(),
                description: Some("Check the project for errors".to_string()),
                command: "cargo check".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(300),
                os_filter: None,
            },
            SuggestedCommand {
                name: "cargo-run".to_string(),
                description: Some("Run the project".to_string()),
                command: "cargo run".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(300),
                os_filter: None,
            },
            SuggestedCommand {
                name: "cargo-clean".to_string(),
                description: Some("Remove build artifacts".to_string()),
                command: "cargo clean".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(60),
                os_filter: None,
            },
        ];

        // Check if it's a binary or library
        let cargo_toml_path = self.project_path.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_toml_path).await {
            if content.contains("[[bin]]") || self.has_file("src/main.rs").await? {
                // Binary project - add clippy and fmt
                commands.push(SuggestedCommand {
                    name: "cargo-clippy".to_string(),
                    description: Some("Run clippy linter".to_string()),
                    command: "cargo clippy".to_string(),
                    shell: Some("bash".to_string()),
                    working_directory: None,
                    environment: None,
                    timeout_seconds: Some(300),
                    os_filter: None,
                });
                commands.push(SuggestedCommand {
                    name: "cargo-fmt".to_string(),
                    description: Some("Format code".to_string()),
                    command: "cargo fmt".to_string(),
                    shell: Some("bash".to_string()),
                    working_directory: None,
                    environment: None,
                    timeout_seconds: Some(60),
                    os_filter: None,
                });
            }
        }

        info!("Discovered {} cargo commands", commands.len());
        Ok(commands)
    }

    /// Discover Python commands
    pub async fn discover_python_commands(&self) -> AppResult<Vec<SuggestedCommand>> {
        let tool = match self.detect_python_tool().await? {
            Some(t) => t,
            None => return Ok(Vec::new()),
        };

        let mut commands = Vec::new();

        match tool {
            PythonTool::Poetry => {
                commands.extend(vec![
                    SuggestedCommand {
                        name: "poetry-install".to_string(),
                        description: Some("Install dependencies with Poetry".to_string()),
                        command: "poetry install".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(300),
                        os_filter: None,
                    },
                    SuggestedCommand {
                        name: "poetry-run".to_string(),
                        description: Some("Run command in Poetry environment".to_string()),
                        command: "poetry run python".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(60),
                        os_filter: None,
                    },
                ]);
            }
            PythonTool::Pipenv => {
                commands.extend(vec![
                    SuggestedCommand {
                        name: "pipenv-install".to_string(),
                        description: Some("Install dependencies with Pipenv".to_string()),
                        command: "pipenv install".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(300),
                        os_filter: None,
                    },
                    SuggestedCommand {
                        name: "pipenv-run".to_string(),
                        description: Some("Run command in Pipenv environment".to_string()),
                        command: "pipenv run python".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(60),
                        os_filter: None,
                    },
                ]);
            }
            PythonTool::Pip => {
                commands.push(SuggestedCommand {
                    name: "pip-install".to_string(),
                    description: Some("Install dependencies with pip".to_string()),
                    command: "pip install -r requirements.txt".to_string(),
                    shell: Some("bash".to_string()),
                    working_directory: None,
                    environment: None,
                    timeout_seconds: Some(300),
                    os_filter: None,
                });
            }
            PythonTool::Conda => {
                commands.extend(vec![
                    SuggestedCommand {
                        name: "conda-install".to_string(),
                        description: Some("Install dependencies with Conda".to_string()),
                        command: "conda install --file requirements.txt".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(300),
                        os_filter: None,
                    },
                ]);
            }
        }

        // Check for Django
        if self.has_file("manage.py").await? {
            commands.extend(vec![
                SuggestedCommand {
                    name: "django-runserver".to_string(),
                    description: Some("Start Django development server".to_string()),
                    command: "python manage.py runserver".to_string(),
                    shell: Some("bash".to_string()),
                    working_directory: None,
                    environment: Some(HashMap::from([
                        ("DJANGO_SETTINGS_MODULE".to_string(), "settings".to_string()),
                    ])),
                    timeout_seconds: Some(300),
                    os_filter: None,
                },
                SuggestedCommand {
                    name: "django-migrate".to_string(),
                    description: Some("Run Django database migrations".to_string()),
                    command: "python manage.py migrate".to_string(),
                    shell: Some("bash".to_string()),
                    working_directory: None,
                    environment: None,
                    timeout_seconds: Some(300),
                    os_filter: None,
                },
                SuggestedCommand {
                    name: "django-makemigrations".to_string(),
                    description: Some("Create Django database migrations".to_string()),
                    command: "python manage.py makemigrations".to_string(),
                    shell: Some("bash".to_string()),
                    working_directory: None,
                    environment: None,
                    timeout_seconds: Some(60),
                    os_filter: None,
                },
            ]);
        }

        // Check for pytest
        if self.has_file("pytest.ini").await?
            || self.has_file("pyproject.toml").await?
            || self.has_directory("tests").await?
        {
            commands.push(SuggestedCommand {
                name: "pytest".to_string(),
                description: Some("Run tests with pytest".to_string()),
                command: "pytest".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(600),
                os_filter: None,
            });
        }

        info!("Discovered {} python commands", commands.len());
        Ok(commands)
    }

    /// Discover Go commands
    pub async fn discover_go_commands(&self) -> AppResult<Vec<SuggestedCommand>> {
        if !self.has_file("go.mod").await? {
            return Ok(Vec::new());
        }

        let commands = vec![
            SuggestedCommand {
                name: "go-build".to_string(),
                description: Some("Build Go project".to_string()),
                command: "go build ./...".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(300),
                os_filter: None,
            },
            SuggestedCommand {
                name: "go-test".to_string(),
                description: Some("Run Go tests".to_string()),
                command: "go test ./...".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(600),
                os_filter: None,
            },
            SuggestedCommand {
                name: "go-run".to_string(),
                description: Some("Run Go project".to_string()),
                command: "go run .".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(300),
                os_filter: None,
            },
            SuggestedCommand {
                name: "go-mod-tidy".to_string(),
                description: Some("Tidy Go module dependencies".to_string()),
                command: "go mod tidy".to_string(),
                shell: Some("bash".to_string()),
                working_directory: None,
                environment: None,
                timeout_seconds: Some(60),
                os_filter: None,
            },
        ];

        info!("Discovered {} go commands", commands.len());
        Ok(commands)
    }

    /// Discover Java commands
    pub async fn discover_java_commands(&self) -> AppResult<Vec<SuggestedCommand>> {
        let build_tool = match self.detect_java_build_tool().await? {
            Some(t) => t,
            None => return Ok(Vec::new()),
        };

        let commands = match build_tool {
            JavaBuildTool::Maven => {
                vec![
                    SuggestedCommand {
                        name: "mvn-clean".to_string(),
                        description: Some("Clean Maven project".to_string()),
                        command: "mvn clean".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(120),
                        os_filter: None,
                    },
                    SuggestedCommand {
                        name: "mvn-compile".to_string(),
                        description: Some("Compile Maven project".to_string()),
                        command: "mvn compile".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(300),
                        os_filter: None,
                    },
                    SuggestedCommand {
                        name: "mvn-test".to_string(),
                        description: Some("Run Maven tests".to_string()),
                        command: "mvn test".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(600),
                        os_filter: None,
                    },
                    SuggestedCommand {
                        name: "mvn-package".to_string(),
                        description: Some("Package Maven project".to_string()),
                        command: "mvn package".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(600),
                        os_filter: None,
                    },
                ]
            }
            JavaBuildTool::Gradle => {
                vec![
                    SuggestedCommand {
                        name: "gradle-build".to_string(),
                        description: Some("Build Gradle project".to_string()),
                        command: "./gradlew build".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(600),
                        os_filter: None,
                    },
                    SuggestedCommand {
                        name: "gradle-test".to_string(),
                        description: Some("Run Gradle tests".to_string()),
                        command: "./gradlew test".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(600),
                        os_filter: None,
                    },
                    SuggestedCommand {
                        name: "gradle-clean".to_string(),
                        description: Some("Clean Gradle project".to_string()),
                        command: "./gradlew clean".to_string(),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(120),
                        os_filter: None,
                    },
                ]
            }
        };

        info!("Discovered {} java commands", commands.len());
        Ok(commands)
    }

    /// Parse Makefile targets
    pub async fn discover_make_commands(&self) -> AppResult<Vec<SuggestedCommand>> {
        if !self.has_file("Makefile").await? && !self.has_file("makefile").await? {
            return Ok(Vec::new());
        }

        let makefile_path = if self.has_file("Makefile").await? {
            self.project_path.join("Makefile")
        } else {
            self.project_path.join("makefile")
        };

        let content = fs::read_to_string(&makefile_path).await?;
        let mut commands = Vec::new();

        // Parse Makefile targets (simplified parser)
        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }

            // Look for target definitions (line starting with identifier and containing ':')
            if let Some(colon_pos) = trimmed.find(':') {
                let target = trimmed[..colon_pos].trim();

                // Skip special targets and those with variables
                if !target.is_empty()
                    && !target.contains('$')
                    && !target.contains('%')
                    && !target.starts_with('.')
                {
                    commands.push(SuggestedCommand {
                        name: format!("make-{}", target),
                        description: Some(format!("Run make target: {}", target)),
                        command: format!("make {}", target),
                        shell: Some("bash".to_string()),
                        working_directory: None,
                        environment: None,
                        timeout_seconds: Some(300),
                        os_filter: None,
                    });
                }
            }
        }

        info!("Discovered {} make commands", commands.len());
        Ok(commands)
    }

    // Helper methods

    async fn has_file(&self, name: &str) -> AppResult<bool> {
        let path = self.project_path.join(name);
        Ok(path.exists() && path.is_file())
    }

    async fn has_directory(&self, name: &str) -> AppResult<bool> {
        let path = self.project_path.join(name);
        Ok(path.exists() && path.is_dir())
    }

    async fn detect_package_manager(&self) -> AppResult<PackageManager> {
        if self.has_file("bun.lockb").await? {
            Ok(PackageManager::Bun)
        } else if self.has_file("pnpm-lock.yaml").await? {
            Ok(PackageManager::Pnpm)
        } else if self.has_file("yarn.lock").await? {
            Ok(PackageManager::Yarn)
        } else {
            Ok(PackageManager::Npm)
        }
    }

    async fn detect_python_tool(&self) -> AppResult<Option<PythonTool>> {
        if self.has_file("pyproject.toml").await? {
            Ok(Some(PythonTool::Poetry))
        } else if self.has_file("Pipfile").await? {
            Ok(Some(PythonTool::Pipenv))
        } else if self.has_file("environment.yml").await? {
            Ok(Some(PythonTool::Conda))
        } else if self.has_file("requirements.txt").await? {
            Ok(Some(PythonTool::Pip))
        } else {
            Ok(None)
        }
    }

    async fn detect_java_build_tool(&self) -> AppResult<Option<JavaBuildTool>> {
        if self.has_file("pom.xml").await? {
            Ok(Some(JavaBuildTool::Maven))
        } else if self.has_file("build.gradle").await? || self.has_file("build.gradle.kts").await? {
            Ok(Some(JavaBuildTool::Gradle))
        } else {
            Ok(None)
        }
    }

    fn infer_script_description(name: &str, _command: &str) -> String {
        match name {
            "dev" | "start:dev" | "serve" => "Start development server".to_string(),
            "build" => "Build for production".to_string(),
            "start" => "Start production server".to_string(),
            "test" => "Run tests".to_string(),
            "lint" => "Run linter".to_string(),
            "format" | "fmt" => "Format code".to_string(),
            "clean" => "Clean build artifacts".to_string(),
            _ => format!("Run {} script", name),
        }
    }

    fn infer_environment(script_name: &str) -> Option<HashMap<String, String>> {
        match script_name {
            "dev" | "start:dev" => Some(HashMap::from([
                ("NODE_ENV".to_string(), "development".to_string()),
            ])),
            "build" | "start" => Some(HashMap::from([
                ("NODE_ENV".to_string(), "production".to_string()),
            ])),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_nodejs_project() {
        // This would require test fixtures
        // Placeholder for now
    }
}
