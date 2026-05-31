use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

use nocodo_agents::RustEngineerAgent;

pub static LIVE_TEST_LOCK: Mutex<()> = Mutex::new(());

pub struct LiveTestConfig {
    models: String,
    template_path: PathBuf,
    project_path: PathBuf,
    base_url: String,
}

impl LiveTestConfig {
    pub fn from_env() -> Self {
        let models = required_env("RUST_ENGINEER_TEST_MODELS");
        assert!(
            models.split(',').any(|model| !model.trim().is_empty()),
            "RUST_ENGINEER_TEST_MODELS must contain at least one model"
        );

        Self {
            models,
            template_path: PathBuf::from(required_env(
                "RUST_ENGINEER_TEST_PROJECT_TEMPLATE_PATH",
            )),
            project_path: PathBuf::from(required_env("RUST_ENGINEER_TEST_PROJECT_PATH")),
            base_url: required_env("LLAMA_CPP_BASE_URL"),
        }
    }

    pub fn models(&self) -> impl Iterator<Item = &str> {
        self.models
            .split(',')
            .map(str::trim)
            .filter(|model| !model.is_empty())
    }
}

pub fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("missing required env var `{name}`"))
}

pub fn recreate_project_from_template(cfg: &LiveTestConfig) {
    assert!(
        cfg.template_path.is_dir(),
        "RUST_ENGINEER_TEST_PROJECT_TEMPLATE_PATH is not a directory: {}",
        cfg.template_path.display()
    );
    assert_safe_recreate_path(&cfg.project_path, &cfg.template_path);

    if cfg.project_path.exists() {
        fs::remove_dir_all(&cfg.project_path).unwrap_or_else(|e| {
            panic!(
                "failed to delete RUST_ENGINEER_TEST_PROJECT_PATH `{}`: {e}",
                cfg.project_path.display()
            )
        });
    }

    let template_url = local_file_url(&cfg.template_path);
    let status = Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(&template_url)
        .arg(&cfg.project_path)
        .status()
        .unwrap_or_else(|e| panic!("failed to run git clone for live test fixture: {e}"));

    assert!(
        status.success(),
        "git clone failed for live test fixture: {} -> {}",
        cfg.template_path.display(),
        cfg.project_path.display()
    );
}

pub fn local_file_url(path: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|e| {
        panic!(
            "failed to canonicalize template path `{}`: {e}",
            path.display()
        )
    });
    format!("file://{}", path.display())
}

pub fn assert_safe_recreate_path(project_path: &Path, template_path: &Path) {
    assert!(
        project_path.is_absolute(),
        "RUST_ENGINEER_TEST_PROJECT_PATH must be absolute: {}",
        project_path.display()
    );
    assert!(
        project_path.parent().is_some(),
        "RUST_ENGINEER_TEST_PROJECT_PATH must have a parent: {}",
        project_path.display()
    );
    assert!(
        project_path != Path::new("/"),
        "refusing to delete filesystem root"
    );
    assert!(
        project_path != template_path,
        "RUST_ENGINEER_TEST_PROJECT_PATH must differ from RUST_ENGINEER_TEST_PROJECT_TEMPLATE_PATH"
    );
}

pub fn agent_for_model(cfg: &LiveTestConfig, model: &str) -> RustEngineerAgent {
    RustEngineerAgent::new(
        model.to_string(),
        Some(cfg.base_url.clone()),
        cfg.project_path.clone(),
    )
    .unwrap_or_else(|e| panic!("failed to build RustEngineerAgent for model `{model}`: {e}"))
}

pub fn assert_not_empty(model: &str, value: &str, label: &str) {
    assert!(
        !value.trim().is_empty(),
        "model `{model}` returned empty {label}"
    );
}

pub fn assert_clean_code(model: &str, code: &str) {
    assert!(
        !code.contains("```"),
        "model `{model}` leaked markdown fence:\n{code}"
    );
    assert!(
        !code.contains("<think>") && !code.contains("</think>"),
        "model `{model}` leaked reasoning tags:\n{code}"
    );
}

pub fn assert_no_imports(model: &str, code: &str) {
    assert!(
        !code
            .lines()
            .any(|line| line.trim_start().starts_with("use ")),
        "model `{model}` returned imports:\n{code}"
    );
}
