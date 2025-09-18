//! Tests for parsing real GitHub Actions workflows from the nocodo project

#[cfg(test)]
mod real_workflow_tests {
    use crate::parser::WorkflowParser;

    #[allow(unused_imports)]
    use super::*;

    async fn create_workflow_file(
        content: &str,
        filename: &str,
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::tempdir().unwrap();
        let workflow_path = temp_dir.path().join(filename);
        tokio::fs::write(&workflow_path, content).await.unwrap();
        (temp_dir, workflow_path)
    }

    /// Test parsing of api-e2e-tests.yml workflow
    #[tokio::test]
    async fn test_parse_api_e2e_tests_workflow() {
        let workflow_content = r#"
name: API-Only E2E Tests

on:
  push:
    branches: [ main, develop ]
    paths:
      - 'manager-web/src/__tests__/api-e2e/**'
      - 'manager/**'
  pull_request:
    branches: [ main, develop ]

jobs:
  api-e2e-tests:
    name: API E2E Tests
    runs-on: ubuntu-latest

    steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Setup Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Install mold linker
      run: |
        sudo apt-get update
        sudo apt-get install -y mold

    - name: Build nocodo-manager
      run: cargo build --release --bin nocodo-manager

    - name: Install manager-web dependencies
      run: |
        cd manager-web
        npm ci

    - name: Run API E2E Tests
      run: |
        cd manager-web
        npm run test:api-e2e
      env:
        NODE_ENV: test
        TEST_DATABASE_PATH: /tmp/nocodo-test.db
"#;

        let (temp_dir, workflow_path) =
            create_workflow_file(workflow_content, "api-e2e-tests.yml").await;

        let (info, commands) = WorkflowParser::parse_workflow_file(&workflow_path, temp_dir.path())
            .unwrap();

        assert_eq!(info.name, "API-Only E2E Tests");
        assert_eq!(info.jobs_count, 1);
        assert_eq!(info.commands_count, 4); // 4 run steps (including checkout, setup rust, install mold, build, install deps, run tests)

        // Check specific commands
        let command_names: Vec<String> = commands
            .iter()
            .map(|c| c.step_name.clone().unwrap_or_default())
            .collect();
        assert!(command_names.contains(&"Install mold linker".to_string()));
        assert!(command_names.contains(&"Build nocodo-manager".to_string()));
        assert!(command_names.contains(&"Run API E2E Tests".to_string()));

        // Check that commands have correct properties
        let mold_command = commands
            .iter()
            .find(|c| c.step_name == Some("Install mold linker".to_string()))
            .unwrap();
        assert!(mold_command.command.starts_with("sudo apt-get update"));
        assert!(mold_command
            .command
            .contains("sudo apt-get install -y mold"));
        assert_eq!(mold_command.shell, None); // Uses default shell

        let e2e_command = commands
            .iter()
            .find(|c| c.step_name == Some("Run API E2E Tests".to_string()))
            .unwrap();
        assert!(e2e_command.command.contains("cd manager-web"));
        assert!(e2e_command.command.contains("npm run test:api-e2e"));
        assert!(e2e_command
            .environment
            .as_ref()
            .unwrap()
            .contains_key("NODE_ENV"));
        assert!(e2e_command
            .environment
            .as_ref()
            .unwrap()
            .contains_key("TEST_DATABASE_PATH"));
    }

    /// Test parsing of integration-ci.yml workflow
    #[tokio::test]
    async fn test_parse_integration_ci_workflow() {
        let workflow_content = r#"
name: Integration CI

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  schedule:
    - cron: '0 2 * * 0'

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
  NODE_VERSION: 'lts/*'

jobs:
  full-build:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Install mold linker
        run: sudo apt-get update && sudo apt-get install -y mold

      - name: Build Rust workspace
        run: |
          cargo build --release --workspace --verbose
          cargo test --workspace --verbose

      - name: Build Web application
        working-directory: manager-web
        run: |
          npm ci
          npm run build

      - name: Verify Web build
        run: test -d manager-web/dist
"#;

        let (temp_dir, workflow_path) =
            create_workflow_file(workflow_content, "integration-ci.yml").await;

        let (info, commands) = WorkflowParser::parse_workflow_file(&workflow_path, temp_dir.path())
            .unwrap();

        assert_eq!(info.name, "Integration CI");
        assert_eq!(info.jobs_count, 1);
        assert_eq!(info.commands_count, 4); // 4 run steps

        // Check working directory handling
        let web_build = commands
            .iter()
            .find(|c| c.step_name == Some("Build Web application".to_string()))
            .unwrap();
        assert!(web_build
            .working_directory
            .as_ref()
            .unwrap()
            .ends_with("manager-web"));
        assert!(web_build.command.contains("npm ci"));
        assert!(web_build.command.contains("npm run build"));

        // Check multi-line commands
        let rust_build = commands
            .iter()
            .find(|c| c.step_name == Some("Build Rust workspace".to_string()))
            .unwrap();
        assert!(rust_build.command.contains("cargo build"));
        assert!(rust_build.command.contains("cargo test"));
    }

    /// Test parsing of web-ci.yml workflow
    #[tokio::test]
    async fn test_parse_web_ci_workflow() {
        let workflow_content = r#"
name: Web CI

on:
  pull_request:
    paths:
      - 'manager-web/**'
      - '.github/workflows/web-ci.yml'
  push:
    branches: [main]

env:
  NODE_VERSION: '20'

jobs:
  typecheck:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: ./manager-web
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}

      - name: Install dependencies
        run: npm ci

      - name: TypeScript type checking
        run: npm run typecheck

  test:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: ./manager-web
    strategy:
      matrix:
        node-version: ['18', '20', 'lts/*']
    steps:
      - name: Setup Node.js ${{ matrix.node-version }}
        uses: actions/setup-node@v4
        with:
          node-version: ${{ matrix.node-version }}

      - name: Install dependencies
        run: npm ci

      - name: Run unit tests
        run: npm run test:run

  build:
    runs-on: ubuntu-latest
    needs: [typecheck, format, lint, test]
    defaults:
      run:
        working-directory: ./manager-web
    steps:
      - name: Install dependencies
        run: npm ci

      - name: Build application
        run: npm run build

      - name: Verify build output
        run: |
          if [ ! -d "dist" ]; then
            echo "Build failed: dist directory not found"
            exit 1
          fi
          if [ ! -f "dist/index.html" ]; then
            echo "Build failed: index.html not found in dist"
            exit 1
          fi
          echo "Build verification passed"
"#;

        let (temp_dir, workflow_path) = create_workflow_file(workflow_content, "web-ci.yml").await;

        let (info, commands) = WorkflowParser::parse_workflow_file(&workflow_path, temp_dir.path())
            
            .unwrap();

        assert_eq!(info.name, "Web CI");
        assert_eq!(info.jobs_count, 3); // typecheck, test, build

        // Check working directory defaults
        let typecheck_commands: Vec<_> = commands
            .iter()
            .filter(|c| c.job_name == "typecheck")
            .collect();
        for cmd in typecheck_commands {
            if let Some(wd) = &cmd.working_directory {
                assert!(wd.ends_with("manager-web"));
            }
        }

        // Check matrix strategy parsing
        let test_commands: Vec<_> = commands.iter().filter(|c| c.job_name == "test").collect();
        assert!(!test_commands.is_empty());
    }

    /// Test parsing of website-ci.yml workflow
    #[tokio::test]
    async fn test_parse_website_ci_workflow() {
        let workflow_content = r#"
name: Website CI

on:
  pull_request:
    paths:
      - 'website/**'
  push:
    branches: [main]

jobs:
  build-and-test:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: ./website

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 'lts/*'

      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: latest

      - name: Install dependencies
        run: |
          pnpm install --frozen-lockfile

      - name: Type check
        run: |
          pnpm astro check

      - name: Build website
        run: |
          pnpm build
"#;

        let (temp_dir, workflow_path) =
            create_workflow_file(workflow_content, "website-ci.yml").await;

        let (info, commands) = WorkflowParser::parse_workflow_file(&workflow_path, temp_dir.path())
            
            .unwrap();

        assert_eq!(info.name, "Website CI");
        assert_eq!(info.jobs_count, 1);
        assert_eq!(info.commands_count, 3); // 3 run steps

        // Check working directory defaults
        for cmd in &commands {
            if let Some(wd) = &cmd.working_directory {
                assert!(wd.ends_with("website"));
            }
        }

        // Check specific commands
        let type_check = commands
            .iter()
            .find(|c| c.step_name == Some("Type check".to_string()))
            .unwrap();
        assert!(type_check.command.trim() == "pnpm astro check");

        let build = commands
            .iter()
            .find(|c| c.step_name == Some("Build website".to_string()))
            .unwrap();
        assert!(build.command.trim() == "pnpm build");
    }

    /// Test parsing multiple workflows from directory
    #[tokio::test]
    async fn test_parse_multiple_workflows_from_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let workflows_dir = temp_dir.path().join(".github").join("workflows");
        std::fs::create_dir_all(&workflows_dir).unwrap();

        // Create multiple workflow files
        let workflow1_content = r#"
name: Workflow 1
on: push
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: echo "test1"
"#;

        let workflow2_content = r#"
name: Workflow 2
on: pull_request
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: echo "build1"
      - run: echo "build2"
"#;

        std::fs::write(workflows_dir.join("workflow1.yml"), workflow1_content).unwrap();
        std::fs::write(workflows_dir.join("workflow2.yaml"), workflow2_content).unwrap();
        std::fs::write(workflows_dir.join("not-a-workflow.txt"), "not yaml").unwrap();

        let workflows = WorkflowParser::scan_workflows_directory(&workflows_dir, temp_dir.path())
            
            .unwrap();

        assert_eq!(workflows.len(), 2);

        let (info1, commands1) = &workflows[0];
        let (info2, commands2) = &workflows[1];

        // Results may be in any order due to directory iteration
        let (wf1_info, wf1_commands, wf2_info, wf2_commands) = if info1.name == "Workflow 1" {
            (info1, commands1, info2, commands2)
        } else {
            (info2, commands2, info1, commands1)
        };

        assert_eq!(wf1_info.name, "Workflow 1");
        assert_eq!(wf1_commands.len(), 1);
        assert_eq!(wf1_commands[0].command, "echo \"test1\"");

        assert_eq!(wf2_info.name, "Workflow 2");
        assert_eq!(wf2_commands.len(), 2);
        assert_eq!(wf2_commands[0].command, "echo \"build1\"");
        assert_eq!(wf2_commands[1].command, "echo \"build2\"");
    }
}
