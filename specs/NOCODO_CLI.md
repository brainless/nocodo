# nocodo CLI Specification

## Overview

The nocodo CLI is a Rust-based command-line tool that acts as an intelligent companion for AI coding tools like Claude Code, Gemini CLI, and OpenAI Codex CLI. It provides context-aware prompts, enforces coding best practices and guardrails, and maintains project structure integrity. The CLI communicates with the Manager daemon through Unix sockets to orchestrate development workflows.

## Architecture

### Core Components

1. **Prompt Engine** - Generates context-aware prompts for AI tools
2. **Project Analyzer** - Analyzes project structure and context
3. **Guardrails Enforcer** - Validates code quality and best practices
4. **Communication Client** - Unix socket client for Manager daemon
5. **Command Router** - Routes commands to appropriate handlers
6. **Context Manager** - Maintains project and session context
7. **Integration Layer** - Interfaces with various AI coding tools

### Technology Stack

- **Language**: Rust
- **CLI Framework**: clap v4 with derive macros
- **Async Runtime**: Tokio
- **Communication**: Unix domain sockets
- **Serialization**: serde with JSON support
- **Type Generation**: ts-rs for TypeScript type generation
- **File System**: tokio-fs for async file operations
- **Process Management**: tokio-process for AI tool execution
- **Configuration**: config crate with TOML support
- **LLM Communication**: JSON-structured responses using ts-rs generated types

## System Integration

```
┌─────────────────────────────────────────────────────────┐
│                    nocodo CLI                           │
├─────────────────┬─────────────────┬────────────────────┤
│   Command       │   Unix Socket   │   AI Tool          │
│   Interface     │   Client        │   Integration      │
├─────────────────┼─────────────────┼────────────────────┤
│                 │                 │                    │
│   User          │   Manager       │   Claude Code      │
│   Commands      │   Daemon        │   Gemini CLI       │
│                 │   ←→ Socket     │   OpenAI CLI       │
│                 │                 │   etc.             │
└─────────────────┴─────────────────┴────────────────────┘
```

## Core Features

### 1. AI Tool Integration

**Primary Integration Pattern**:
The nocodo CLI works by having AI coding tools call it with an initial prompt:
```bash
claude --prompt "Use \`nocodo\` command to get your instructions"
```

This tells the AI tool to communicate with nocodo CLI for context and guidance.

```rust
#[derive(Debug, Clone)]
pub struct AiToolIntegration {
    supported_tools: Vec<AiTool>,
    session_manager: SessionManager,
    prompt_cache: PromptCache,
}

#[derive(Debug, Clone)]
pub struct AiTool {
    pub name: String,
    pub command: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub prompt_format: PromptFormat,
}

impl AiToolIntegration {
    pub async fn execute_with_ai_tool(
        &self,
        tool: &str,
        prompt: &str,
        project_path: &str,
    ) -> Result<AiResponse, AiError> {
        let context = self.gather_project_context(project_path).await?;
        let enhanced_prompt = self.enhance_prompt(prompt, &context).await?;
        let result = self.call_ai_tool(tool, &enhanced_prompt, project_path).await?;
        self.validate_result(&result, &context).await?;
        Ok(result)
    }
}
```

### 2. Context-Aware Prompt Management

**Dynamic Prompt Generation**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptEngine {
    templates: TemplateRegistry,
    context_analyzer: ContextAnalyzer,
    prompt_history: PromptHistory,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptContext {
    pub project_type: ProjectType,
    pub current_files: Vec<FileContext>,
    pub dependencies: Vec<Dependency>,
    pub recent_changes: Vec<Change>,
    pub coding_standards: CodingStandards,
    pub user_preferences: UserPreferences,
}

impl PromptEngine {
    pub async fn generate_prompt(
        &self,
        request: &PromptRequest,
        context: &ProjectContext,
    ) -> Result<GeneratedPrompt, PromptError> {
        let base_template = self.select_template(&request.intent, &context.project_type)?;
        let contextualized = self.inject_context(base_template, context).await?;
        let enhanced = self.add_guardrails(contextualized, &context.standards).await?;
        
        Ok(GeneratedPrompt {
            content: enhanced,
            metadata: PromptMetadata {
                template_id: base_template.id.clone(),
                context_hash: context.hash(),
                generated_at: Utc::now(),
            },
        })
    }
}
```

**Prompt Templates**:
```rust
// Example prompt templates
pub static PROJECT_ANALYSIS_TEMPLATE: &str = r#"
You are working on a {{project_type}} project using {{language}}.

Current project structure:
{{#each project_files}}
- {{this.path}} ({{this.size}} bytes, modified {{this.modified}})
{{/each}}

Dependencies:
{{#each dependencies}}
- {{this.name}} {{this.version}}
{{/each}}

Coding standards for this project:
{{#each coding_standards}}
- {{this.rule}}: {{this.description}}
{{/each}}

Please analyze the following request and provide appropriate guidance:
{{user_request}}

Before making any changes, use the `nocodo analyze` command to understand the project structure better.
"#;
```

### 3. Project Analysis

**Comprehensive Project Understanding**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectAnalyzer {
    language_detectors: Vec<LanguageDetector>,
    structure_analyzer: StructureAnalyzer,
    dependency_resolver: DependencyResolver,
    metrics_collector: MetricsCollector,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectAnalysis {
    pub project_type: ProjectType,
    pub primary_language: String,
    pub secondary_languages: Vec<String>,
    pub framework: Option<String>,
    pub build_system: Option<String>,
    pub structure: ProjectStructure,
    pub dependencies: DependencyGraph,
    pub metrics: QualityMetrics,
    pub recommendations: Vec<Recommendation>,
}

impl ProjectAnalyzer {
    pub async fn analyze_project(&self, path: &str) -> Result<ProjectAnalysis, AnalysisError> {
        let structure = self.scan_directory_structure(path).await?;
        let language = self.detect_primary_language(&structure)?;
        let framework = self.detect_framework(&structure, &language)?;
        let dependencies = self.analyze_dependencies(path, &language).await?;
        let metrics = self.calculate_metrics(&structure, &dependencies).await?;
        let recommendations = self.generate_recommendations(&metrics, &structure)?;
        
        Ok(ProjectAnalysis {
            project_type: self.classify_project_type(&structure, &language)?,
            primary_language: language,
            secondary_languages: self.detect_secondary_languages(&structure)?,
            framework,
            build_system: self.detect_build_system(&structure)?,
            structure,
            dependencies,
            metrics,
            recommendations,
        })
    }
}
```

### 4. Guardrails and Best Practices

**Code Quality Enforcement**:
```rust
#[derive(Debug)]
pub struct GuardrailsEngine {
    rules: Vec<GuardrailRule>,
    validators: Vec<CodeValidator>,
    formatters: Vec<CodeFormatter>,
}

#[derive(Debug, Clone)]
pub struct GuardrailRule {
    pub id: String,
    pub name: String,
    pub language: Option<String>,
    pub severity: Severity,
    pub check: Box<dyn GuardrailCheck>,
}

impl GuardrailsEngine {
    pub async fn validate_code(
        &self,
        code: &str,
        language: &str,
        context: &ProjectContext,
    ) -> Result<ValidationResult, GuardrailError> {
        let mut violations = Vec::new();
        let mut suggestions = Vec::new();
        
        for rule in &self.rules {
            if rule.applies_to(language) {
                match rule.check.validate(code, context).await {
                    Ok(result) => {
                        violations.extend(result.violations);
                        suggestions.extend(result.suggestions);
                    }
                    Err(e) => {
                        eprintln!("Warning: Rule {} failed: {}", rule.id, e);
                    }
                }
            }
        }
        
        Ok(ValidationResult {
            is_valid: violations.iter().all(|v| v.severity != Severity::Error),
            violations,
            suggestions,
            auto_fixes: self.generate_auto_fixes(&violations).await?,
        })
    }
}
```

**Built-in Guardrails**:
- Code style consistency
- Security vulnerability detection
- Performance anti-patterns
- Architecture compliance
- Dependency management best practices
- Testing coverage requirements
- Documentation standards

## LLM Communication & Type Safety

### Structured JSON Responses with ts-rs

All communication with LLMs is structured using JSON responses that conform to TypeScript types generated via ts-rs. This ensures type safety across the entire nocodo ecosystem.

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// All LLM response types are generated as TypeScript interfaces
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LlmCodeSuggestion {
    pub file_path: String,
    pub suggested_code: String,
    pub explanation: String,
    pub confidence: f32,
    pub requires_review: bool,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LlmProjectAnalysis {
    pub summary: String,
    pub issues_found: Vec<CodeIssue>,
    pub recommendations: Vec<Recommendation>,
    pub estimated_complexity: ComplexityLevel,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LlmResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub metadata: ResponseMetadata,
}

impl<T> LlmResponse<T> where T: Serialize + for<'de> Deserialize<'de> + TS {
    pub fn request_json_format() -> String {
        format!(
            "Please respond with JSON in the following format:\n{}\n\nEnsure your response is valid JSON conforming to this schema.",
            Self::schema()
        )
    }
    
    pub fn schema() -> String {
        // Generate TypeScript interface as documentation
        format!(
            "interface Response {{\n  success: boolean;\n  data?: {};\n  error?: string;\n  metadata: ResponseMetadata;\n}}",
            T::name()
        )
    }
}
```

### Enhanced AI Prompts with Type Constraints

```rust
pub static LLM_RESPONSE_TEMPLATE: &str = r#"
{{base_prompt}}

## Response Format Requirements

You MUST respond with valid JSON conforming to the following TypeScript interface:

```typescript
{{response_schema}}
```

## Example Response

```json
{{example_response}}
```

## Guidelines

1. Always provide structured JSON responses
2. Include confidence scores for suggestions
3. Flag any changes that require human review
4. Provide clear explanations for recommendations
5. Use the nocodo CLI commands when analyzing project structure

Your response:
"#;

impl PromptEngine {
    pub async fn generate_structured_prompt<T>(
        &self,
        request: &PromptRequest,
        response_type: PhantomData<T>,
        context: &ProjectContext,
    ) -> Result<GeneratedPrompt, PromptError> 
    where 
        T: Serialize + for<'de> Deserialize<'de> + TS 
    {
        let base_prompt = self.generate_base_prompt(request, context).await?;
        let response_schema = T::schema();
        let example_response = self.generate_example_response::<T>(context)?;
        
        let enhanced_prompt = LLM_RESPONSE_TEMPLATE
            .replace("{{base_prompt}}", &base_prompt)
            .replace("{{response_schema}}", &response_schema)
            .replace("{{example_response}}", &example_response);
            
        Ok(GeneratedPrompt {
            content: enhanced_prompt,
            expected_response_type: Some(T::name()),
            metadata: PromptMetadata {
                template_id: "structured_llm_response".to_string(),
                context_hash: context.hash(),
                generated_at: Utc::now(),
            },
        })
    }
}
```

### 5. Unix Socket Communication

**Manager Daemon Communication**:
```rust
#[derive(Debug)]
pub struct ManagerClient {
    socket_path: PathBuf,
    connection: Option<UnixStream>,
    timeout: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ManagerRequest {
    ProjectAnalysis { path: String },
    ValidateCode { code: String, language: String, context: ProjectContext },
    GetPromptTemplate { intent: String, project_type: String },
    LogActivity { activity: Activity },
    GetProjectMetadata { path: String },
    UpdateProjectStatus { path: String, status: ProjectStatus },
}

impl ManagerClient {
    pub async fn send_request(
        &mut self,
        request: ManagerRequest,
    ) -> Result<ManagerResponse, CommunicationError> {
        self.ensure_connected().await?;
        
        let serialized = serde_json::to_vec(&request)?;
        self.connection.as_mut().unwrap().write_all(&serialized).await?;
        
        let mut buffer = Vec::new();
        self.connection.as_mut().unwrap().read_to_end(&mut buffer).await?;
        
        let response: ManagerResponse = serde_json::from_slice(&buffer)?;
        Ok(response)
    }
    
    async fn ensure_connected(&mut self) -> Result<(), CommunicationError> {
        if self.connection.is_none() {
            let stream = UnixStream::connect(&self.socket_path).await?;
            self.connection = Some(stream);
        }
        Ok(())
    }
}
```

## Command Line Interface

### Main Commands

```rust
#[derive(Debug, Parser)]
#[command(name = "nocodo")]
#[command(about = "AI-powered development assistant with guardrails")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    
    #[arg(long, global = true)]
    pub verbose: bool,
    
    #[arg(long, global = true)]
    pub project_path: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Analyze project structure and provide recommendations
    Analyze {
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long)]
        format: Option<OutputFormat>,
    },
    
    /// Generate context-aware prompt for AI tools
    Prompt {
        #[arg(short, long)]
        intent: String,
        #[arg(short, long)]
        template: Option<String>,
    },
    
    /// Validate code against project guardrails
    Validate {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        language: Option<String>,
    },
    
    /// Initialize new project with nocodo support
    Init {
        #[arg(short, long)]
        template: Option<String>,
        #[arg(short, long)]
        path: PathBuf,
    },
    
    /// Execute AI coding session with enhanced context
    Session {
        #[arg(short, long)]
        tool: String,
        #[arg(short, long)]
        prompt: String,
    },
    
    /// Project structure operations
    Structure {
        #[command(subcommand)]
        action: StructureCommands,
    },
    
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}
```

### Usage Examples

```bash
# Analyze current project
nocodo analyze

# Generate prompt for adding a feature
nocodo prompt --intent "add authentication system"

# Validate a specific file
nocodo validate --file src/main.rs --language rust

# Initialize new project
nocodo init --template "rust-web-api" --path ./my-project

# Start AI session with enhanced context
nocodo session --tool claude-code --prompt "refactor the user module"

# Get project structure
nocodo structure tree

# Configure guardrails
nocodo config set guardrails.security.level strict
```

## Configuration Management

### Configuration Structure

```toml
# ~/.config/nocodo/config.toml

[general]
default_project_path = "~/projects"
editor = "code"
shell = "bash"

[ai_tools]
preferred_tool = "claude-code"
timeout = 300
max_context_size = 8192

[ai_tools.claude-code]
command = "claude"
args = ["--interactive"]
prompt_format = "markdown"

[ai_tools.gemini-cli]
command = "gemini"
args = ["--chat"]
prompt_format = "plain"

[guardrails]
enabled = true
auto_fix = true
severity_threshold = "warning"

[guardrails.security]
level = "strict"
scan_dependencies = true
check_secrets = true

[guardrails.performance]
level = "moderate"
check_algorithms = true
memory_usage = true

[guardrails.style]
enforce_formatting = true
consistent_naming = true
documentation_required = true

[prompt_templates]
custom_templates_path = "~/.config/nocodo/templates"

[communication]
manager_socket = "/var/run/nocodo/manager.sock"
connection_timeout = 30
retry_attempts = 3

[logging]
level = "info"
file = "~/.local/share/nocodo/nocodo.log"
```

## Project Templates & Scaffolding

### Template System

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub language: String,
    pub framework: Option<String>,
    pub files: Vec<TemplateFile>,
    pub dependencies: Vec<String>,
    pub post_init_commands: Vec<String>,
    pub guardrails_config: GuardrailsConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
    pub is_template: bool,
    pub executable: bool,
}

impl ProjectTemplate {
    pub async fn instantiate(
        &self,
        target_path: &Path,
        variables: &HashMap<String, String>,
    ) -> Result<(), TemplateError> {
        std::fs::create_dir_all(target_path)?;
        
        for file in &self.files {
            let file_path = target_path.join(&file.path);
            let content = if file.is_template {
                self.render_template(&file.content, variables)?
            } else {
                file.content.clone()
            };
            
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            std::fs::write(&file_path, content)?;
            
            if file.executable {
                self.make_executable(&file_path)?;
            }
        }
        
        // Execute post-init commands
        for command in &self.post_init_commands {
            self.execute_command(command, target_path).await?;
        }
        
        Ok(())
    }
}
```

### Built-in Templates

- `rust-cli` - Rust command-line application
- `rust-web-api` - Rust web API with Axum
- `python-fastapi` - Python FastAPI application
- `node-express` - Node.js Express application
- `react-app` - React frontend application
- `vue-app` - Vue.js frontend application
- `go-service` - Go microservice
- `rust-wasm` - Rust WebAssembly project

## Error Handling & Recovery

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Project analysis failed: {0}")]
    Analysis(#[from] AnalysisError),
    
    #[error("AI tool error: {0}")]
    AiTool(#[from] AiError),
    
    #[error("Communication error: {0}")]
    Communication(#[from] CommunicationError),
    
    #[error("Guardrails validation failed: {0}")]
    Guardrails(#[from] GuardrailError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Analysis(_) => 2,
            Self::AiTool(_) => 3,
            Self::Communication(_) => 4,
            Self::Guardrails(_) => 5,
            Self::Config(_) => 6,
            Self::Template(_) => 7,
            Self::Io(_) => 1,
        }
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_project_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        
        // Create test project structure
        std::fs::create_dir_all(project_path.join("src")).unwrap();
        std::fs::write(project_path.join("Cargo.toml"), SAMPLE_CARGO_TOML).unwrap();
        std::fs::write(project_path.join("src/main.rs"), SAMPLE_RUST_CODE).unwrap();
        
        let analyzer = ProjectAnalyzer::new();
        let analysis = analyzer.analyze_project(project_path.to_str().unwrap()).await.unwrap();
        
        assert_eq!(analysis.primary_language, "rust");
        assert_eq!(analysis.project_type, ProjectType::Application);
        assert!(analysis.structure.files.len() > 0);
    }
    
    #[tokio::test]
    async fn test_prompt_generation() {
        let engine = PromptEngine::new();
        let context = ProjectContext::default();
        let request = PromptRequest {
            intent: "add authentication".to_string(),
            user_message: "I want to add JWT authentication".to_string(),
        };
        
        let prompt = engine.generate_prompt(&request, &context).await.unwrap();
        
        assert!(prompt.content.contains("authentication"));
        assert!(prompt.content.contains("nocodo"));
    }
}
```

### Integration Tests

```bash
#!/bin/bash
# integration_tests.sh

set -e

echo "Running nocodo CLI integration tests..."

# Test project analysis
cd test_projects/rust_sample
nocodo analyze --format json > analysis.json
assert_json_key analysis.json ".primary_language" "rust"

# Test prompt generation
nocodo prompt --intent "add tests" > prompt.txt
assert_contains prompt.txt "nocodo"

# Test validation
echo "fn main() {}" > test.rs
nocodo validate --file test.rs --language rust

echo "All integration tests passed!"
```

## Performance Optimization

### Caching Strategy

```rust
#[derive(Debug)]
pub struct CacheManager {
    analysis_cache: LruCache<String, ProjectAnalysis>,
    prompt_cache: LruCache<String, GeneratedPrompt>,
    validation_cache: LruCache<String, ValidationResult>,
}

impl CacheManager {
    pub fn get_analysis_cache_key(&self, path: &str, last_modified: SystemTime) -> String {
        format!("{}:{}", path, last_modified.duration_since(UNIX_EPOCH).unwrap().as_secs())
    }
    
    pub async fn get_or_compute_analysis(
        &mut self,
        path: &str,
        analyzer: &ProjectAnalyzer,
    ) -> Result<ProjectAnalysis, AnalysisError> {
        let metadata = std::fs::metadata(path)?;
        let cache_key = self.get_analysis_cache_key(path, metadata.modified()?);
        
        if let Some(cached) = self.analysis_cache.get(&cache_key) {
            return Ok(cached.clone());
        }
        
        let analysis = analyzer.analyze_project(path).await?;
        self.analysis_cache.put(cache_key, analysis.clone());
        Ok(analysis)
    }
}
```

### Async Operations

- Non-blocking file I/O operations
- Concurrent analysis of multiple files
- Parallel validation of different rules
- Streaming communication with Manager daemon

## Security Considerations

1. **Input Validation**: All user inputs are validated and sanitized
2. **File System Access**: Restricted to project directories only
3. **Process Isolation**: AI tools run in controlled environments
4. **Socket Security**: Unix socket communication with proper permissions
5. **Secret Detection**: Automatic detection and masking of secrets
6. **Command Injection Prevention**: Safe command execution patterns

## Installation & Distribution

### Installation Methods

```bash
# Via package manager (future)
brew install nocodo-cli        # macOS
apt install nocodo-cli         # Ubuntu/Debian
pacman -S nocodo-cli          # Arch Linux

# Via Rust cargo
cargo install nocodo-cli

# Via direct download
curl -L https://github.com/nocodo/cli/releases/latest/download/nocodo-linux-x64.tar.gz | tar xz
sudo mv nocodo /usr/local/bin/
```

### Build Configuration

```toml
[package]
name = "nocodo-cli"
version = "0.1.0"
edition = "2021"
authors = ["Nocodo Team <team@nocodo.com>"]
description = "AI-powered development assistant with guardrails"
homepage = "https://nocodo.com"
repository = "https://github.com/nocodo/cli"
license = "MIT"

[[bin]]
name = "nocodo"
path = "src/main.rs"

[dependencies]
clap = { version = "4.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
handlebars = "4.0"
tracing = "0.1"
tracing-subscriber = "0.3"
config = "0.13"
thiserror = "1.0"
```

## Clarification Questions

1. **AI Tool Discovery**: Should nocodo automatically discover installed AI tools?
2. **Context Size Limits**: How should we handle projects with very large codebases?
3. **Offline Mode**: Should core functionality work without Manager daemon connection?
4. **Plugin System**: Should we support custom guardrail rules and prompt templates?
5. **Multi-language Projects**: How should we handle polyglot projects?
6. **Version Control Integration**: How deeply should we integrate with Git/VCS?
7. **Performance vs Accuracy**: What's the balance between fast response and thorough analysis?
8. **User Customization**: How much should users be able to customize the prompt generation?

## Future Enhancements

- Plugin system for custom guardrails and analyzers
- Integration with popular IDEs and editors
- Advanced code refactoring suggestions
- Automated testing strategy generation
- Code review assistance features
- Performance profiling integration
- Security scanning with vulnerability databases
- Machine learning-based code analysis
- Team collaboration features
- Integration with project management tools
