# Real LLM Integration Test Plan for Issue #133

## 🚨 Critical Issue Identified

After reviewing the migrated Rust tests, a **serious gap** was found: tests use hardcoded LLM responses instead of making real API calls to LLM providers.

### Current Problem
- Tests contain hardcoded conversations like `("assistant", "Yes, I'd be happy to help you with Rust!")`
- `llm_agent: None` in test configuration bypasses real LLM integration
- Tests validate API endpoints but not actual LLM functionality

### Required Change
Tests must make **real API calls** to LLM providers (Grok Code Fast 1, OpenAI, Anthropic) and validate actual responses using keyword-based validation.

## 🎯 Solution: Keyword-Based Response Validation

To address non-deterministic LLM responses while ensuring real integration:

**Example**: When asking for "tech stack analysis" of a Python FastAPI + React project, validate that the response contains:
- **Required keywords**: "Python", "FastAPI", "React"
- **Optional keywords**: "TypeScript", "full-stack", "API"
- **Forbidden keywords**: "Django", "Vue", "Java"

## 🏗️ Implementation Plan

### Phase 1: Multi-Provider Test Configuration (Days 1-2)

#### Enhanced Test Configuration
```rust
// tests/common/llm_config.rs
pub struct LlmTestConfig {
    pub enabled_providers: Vec<LlmProviderTestConfig>,
    pub default_provider: Option<String>,
    pub test_timeouts: LlmTestTimeouts,
}

pub struct LlmProviderTestConfig {
    pub name: String,           // "grok", "openai", "anthropic"
    pub models: Vec<String>,    // ["grok-code-fast-1", "gpt-4", "claude-3"]
    pub api_key_env: String,    // "GROK_API_KEY", "OPENAI_API_KEY"
    pub enabled: bool,          // Skip if API key not available
    pub test_prompts: LlmTestPrompts,
}
```

#### Environment-Based Provider Detection
```rust
impl LlmTestConfig {
    pub fn from_environment() -> Self {
        let mut providers = Vec::new();

        // Auto-detect available API keys
        if env::var("GROK_API_KEY").is_ok() {
            providers.push(LlmProviderTestConfig::grok());
        }
        if env::var("OPENAI_API_KEY").is_ok() {
            providers.push(LlmProviderTestConfig::openai());
        }
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            providers.push(LlmProviderTestConfig::anthropic());
        }

        Self { providers, default_provider: providers.get(0).map(|p| p.name.clone()), ... }
    }
}
```

### Phase 2: Test Parametrization System (Days 3-4)

#### Provider-Agnostic Test Macros
```rust
// tests/common/llm_test_macros.rs
macro_rules! test_all_llm_providers {
    ($test_name:ident, $test_fn:expr) => {
        mod $test_name {
            // Generate one test per available provider
            async fn run_test_for_provider(provider: &LlmProviderTestConfig) {
                let test_app = TestApp::new_with_llm(provider).await;
                $test_fn(test_app, provider).await;
            }

            generate_provider_tests!(); // Compile-time test generation
        }
    };
}
```

#### Real LLM Integration in TestApp
```rust
impl TestApp {
    pub async fn new_with_llm(provider: &LlmProviderTestConfig) -> Self {
        // Create REAL LLM agent with provider configuration
        let llm_agent = Some(Arc::new(LlmAgent::new(
            database.database.clone(),
            ws_broadcaster.clone(),
            config.projects_dir(),
            Arc::new(provider.to_app_config()),
        )));

        let app_state = web::Data::new(AppState {
            llm_agent, // ← Real LLM agent, not None!
            // ... other fields
        });
    }

    pub async fn send_llm_message(&self, message: String) -> Result<String> {
        // REAL LLM API call - no simulation!
        let response = self.llm_agent()
            .process_message(&session.id, message)
            .await?;
        Ok(response)
    }
}
```

### Phase 3: Keyword-Based Validation Framework (Days 5-6)

#### Test Scenarios with Expected Keywords
```rust
#[derive(Debug, Clone)]
pub struct LlmTestScenario {
    pub name: String,
    pub context: LlmTestContext,      // Project files and structure
    pub prompt: String,               // Question to ask LLM
    pub expected_keywords: LlmKeywordExpectations,
}

#[derive(Debug, Clone)]
pub struct LlmKeywordExpectations {
    pub required_keywords: Vec<String>,   // Must contain ALL of these
    pub optional_keywords: Vec<String>,   // Should contain SOME of these
    pub forbidden_keywords: Vec<String>,  // Must NOT contain these
    pub minimum_score: f32,               // Keyword coverage threshold (0.7)
}
```

#### Predefined Test Scenarios
```rust
impl LlmTestScenario {
    pub fn tech_stack_analysis_python_fastapi() -> Self {
        Self {
            name: "Tech Stack Analysis - Python FastAPI + React".to_string(),
            context: LlmTestContext {
                files: vec![
                    TestFile { path: "requirements.txt", content: "fastapi==0.104.1\nuvicorn==0.24.0", language: "text" },
                    TestFile { path: "main.py", content: "from fastapi import FastAPI\napp = FastAPI()", language: "python" },
                    TestFile { path: "package.json", content: r#"{"dependencies": {"react": "^18.2.0", "typescript": "^5.0.0"}}"#, language: "json" },
                    TestFile { path: "src/App.tsx", content: "import React from 'react';\nfunction App() { return <div>Hello!</div>; }", language: "typescript" },
                ],
            },
            prompt: "Analyze the tech stack of this project. What technologies and frameworks are being used?".to_string(),
            expected_keywords: LlmKeywordExpectations {
                required_keywords: vec!["Python".to_string(), "FastAPI".to_string(), "React".to_string()],
                optional_keywords: vec!["TypeScript".to_string(), "full-stack".to_string(), "API".to_string()],
                forbidden_keywords: vec!["Django".to_string(), "Vue".to_string(), "Java".to_string()],
                minimum_score: 0.7,
            },
        }
    }
}
```

#### Smart Keyword Validation Engine
```rust
pub struct KeywordValidator;

impl KeywordValidator {
    pub fn validate_response(
        response: &str,
        expectations: &LlmKeywordExpectations
    ) -> LlmValidationResult {
        let response_lower = response.to_lowercase();

        // Check required keywords (ALL must be present)
        let found_required: Vec<_> = expectations.required_keywords
            .iter()
            .filter(|k| Self::contains_keyword(&response_lower, k))
            .cloned()
            .collect();

        // Check optional keywords (SOME should be present)
        let found_optional: Vec<_> = expectations.optional_keywords
            .iter()
            .filter(|k| Self::contains_keyword(&response_lower, k))
            .cloned()
            .collect();

        // Check forbidden keywords (NONE should be present)
        let found_forbidden: Vec<_> = expectations.forbidden_keywords
            .iter()
            .filter(|k| Self::contains_keyword(&response_lower, k))
            .cloned()
            .collect();

        let score = Self::calculate_score(&found_required, &found_optional, &found_forbidden, expectations);

        LlmValidationResult {
            passed: found_required.len() == expectations.required_keywords.len()
                && found_forbidden.is_empty()
                && score >= expectations.minimum_score,
            score,
            found_required,
            found_optional,
            found_forbidden,
            missing_required: expectations.required_keywords.iter()
                .filter(|k| !Self::contains_keyword(&response_lower, k))
                .cloned()
                .collect(),
        }
    }

    fn contains_keyword(text: &str, keyword: &str) -> bool {
        let keyword_lower = keyword.to_lowercase();

        // Exact match
        if text.contains(&keyword_lower) {
            return true;
        }

        // Fuzzy matching for common variations
        match keyword_lower.as_str() {
            "fastapi" => text.contains("fast api"),
            "typescript" => text.contains("ts"),
            "javascript" => text.contains("js"),
            "react" => text.contains("reactjs"),
            _ => false,
        }
    }

    fn calculate_score(
        found_required: &[String],
        found_optional: &[String],
        found_forbidden: &[String],
        expectations: &LlmKeywordExpectations
    ) -> f32 {
        let required_score = if expectations.required_keywords.is_empty() {
            1.0
        } else {
            found_required.len() as f32 / expectations.required_keywords.len() as f32
        };

        let optional_score = if expectations.optional_keywords.is_empty() {
            0.0
        } else {
            found_optional.len() as f32 / expectations.optional_keywords.len() as f32
        };

        let forbidden_penalty = found_forbidden.len() as f32 * 0.1;

        ((required_score * 0.6) + (optional_score * 0.3) - forbidden_penalty).max(0.0).min(1.0)
    }
}
```

### Phase 4: Test Implementation (Days 7-8)

#### Real LLM Tests with Keyword Validation
```rust
// tests/integration/llm_keyword_validation.rs
#[test_all_llm_providers]
async fn test_tech_stack_analysis(app: TestApp, provider: &LlmProviderTestConfig) {
    let scenario = LlmTestScenario::tech_stack_analysis_python_fastapi();

    // Set up project context
    app.create_project_from_scenario(&scenario.context).await?;

    // Send prompt to REAL LLM
    let response = app.send_llm_message(&scenario.prompt).await?;

    // Validate response using keyword expectations
    let validation_result = KeywordValidator::validate_response(
        &response,
        &scenario.expected_keywords
    );

    assert!(
        validation_result.passed,
        "LLM response validation failed for provider {}: {}\n\nResponse: {}\n\nFound required: {:?}\nMissing required: {:?}\nFound forbidden: {:?}\nScore: {:.2}",
        provider.name,
        scenario.name,
        response,
        validation_result.found_required,
        validation_result.missing_required,
        validation_result.found_forbidden,
        validation_result.score
    );
}

#[test_all_llm_providers]
async fn test_code_generation_quality(app: TestApp, provider: &LlmProviderTestConfig) {
    let scenario = LlmTestScenario::code_generation_rust_function();

    app.create_project_from_scenario(&scenario.context).await?;
    let response = app.send_llm_message(&scenario.prompt).await?;

    let validation_result = KeywordValidator::validate_response(&response, &scenario.expected_keywords);

    assert!(validation_result.passed, "Code generation failed: score {:.2}", validation_result.score);

    // Additional validation for code responses
    assert!(
        response.contains("```") || response.contains("fn "),
        "Response should contain code block or function syntax"
    );
}
```

### Phase 5: Advanced Features (Days 9-10)

#### Conditional Test Execution
```rust
pub fn should_run_llm_tests() -> bool {
    let has_any_key = env::var("GROK_API_KEY").is_ok() ||
                     env::var("OPENAI_API_KEY").is_ok() ||
                     env::var("ANTHROPIC_API_KEY").is_ok();

    let disabled = env::var("SKIP_LLM_TESTS").is_ok();
    has_any_key && !disabled
}
```

#### Rate Limiting and Cost Control
```rust
pub struct LlmTestRateLimiter {
    last_request: Arc<Mutex<Instant>>,
    min_interval: Duration, // Prevent API rate limiting
}

pub struct LlmTestMonitor {
    pub total_requests: AtomicU64,
    pub total_cost_estimate: AtomicU64, // Track API costs
}
```

## 🎯 Test Scenarios

| Test Category | Example Scenario | Required Keywords | Optional Keywords | Forbidden Keywords |
|---------------|------------------|-------------------|-------------------|-------------------|
| **Tech Stack Analysis** | Python FastAPI + React project | Python, FastAPI, React | TypeScript, full-stack, API | Django, Vue, Java |
| **Code Generation** | "Write Rust factorial function" | fn, factorial | recursion, u64, match | function, def, public |
| **File Analysis** | Dockerfile analysis | Docker, container | Alpine, npm, port | Python, Java, PHP |
| **Architecture Review** | Microservices project | microservice, API, service | Docker, database, REST | monolith, single |

## 📊 Success Metrics

| Metric | Target | Validation Method |
|--------|--------|-------------------|
| **Real LLM Integration** | 100% of LLM tests | No hardcoded responses, actual HTTP calls |
| **Keyword Accuracy** | 90%+ pass rate | Tests consistently pass with real responses |
| **Provider Coverage** | 3+ providers | Grok, OpenAI, Anthropic support |
| **Context Understanding** | 70%+ average score | Keyword-based scoring system |
| **Test Reliability** | <5% flaky tests | Consistent keyword detection across runs |

## 🚀 Implementation Timeline

### Week 1: Foundation (Days 1-4)
- **Day 1-2**: Multi-provider configuration system
- **Day 3-4**: Test parametrization and real LLM integration

### Week 2: Validation Framework (Days 5-8)
- **Day 5-6**: Keyword validation engine and test scenarios
- **Day 7-8**: Replace hardcoded responses with real LLM tests

### Week 3: Polish & Integration (Days 9-10)
- **Day 9**: Rate limiting, cost control, error handling
- **Day 10**: CI/CD integration and documentation

## 🔧 CI/CD Integration

```bash
# Environment variables for CI
export GROK_API_KEY="${{ secrets.GROK_API_KEY }}"
export OPENAI_API_KEY="${{ secrets.OPENAI_API_KEY }}"
export ANTHROPIC_API_KEY="${{ secrets.ANTHROPIC_API_KEY }}"

# Run tests with real LLM integration
cargo test --test llm_integration -- --test-threads=1
```

## 📋 Migration Impact

**Before**: Fast, deterministic tests with hardcoded responses
**After**: Slower, real API calls with keyword-based validation

**Benefits**:
- ✅ Tests actual LLM integration and functionality
- ✅ Catches real integration issues
- ✅ Validates LLM understanding of project context
- ✅ Works across multiple LLM providers

**Trade-offs**:
- ⚠️ Slower test execution (API calls take time)
- ⚠️ Requires API keys and costs money
- ⚠️ Non-deterministic responses (mitigated by keyword validation)

## 🎯 Expected Outcomes

1. **Real LLM Testing**: All LLM-related tests make actual API calls
2. **Reliable Validation**: Keyword-based approach provides consistent test results
3. **Multi-Provider Support**: Tests work with Grok, OpenAI, and Anthropic
4. **Cost-Controlled**: Rate limiting and monitoring prevent excessive API usage
5. **CI-Ready**: Tests can run in CI environment with proper API key management

This plan transforms the current hardcoded test approach into a robust, real-world LLM integration testing system that validates actual AI capabilities while maintaining test reliability through intelligent keyword-based validation.