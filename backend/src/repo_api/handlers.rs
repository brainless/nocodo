use std::path::Path;

const TEMPLATE_REPO_URL: &str = "https://github.com/brainless/rustysolid.git";

/// Clone template repo into project_path.
/// Skips silently if directory already has a git repo.
pub async fn clone_template_repo(project_path: String) {
    let path = Path::new(&project_path);

    // Skip if directory already has a valid git repo
    if path.join(".git").exists() {
        log::info!(
            "Project directory {} already has a git repo, skipping template clone",
            project_path
        );
        return;
    }

    log::info!("Cloning template repo to {}", project_path);

    let output = tokio::process::Command::new("git")
        .args(["clone", TEMPLATE_REPO_URL, &project_path])
        .output()
        .await;

    match output {
        Ok(ref out) if out.status.success() => {
            log::info!("Template cloned successfully to {}", project_path);
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            log::warn!("Template clone failed: {}", stderr.trim());
            // Clean up partial clone so retry isn't blocked by .git check
            let _ = std::fs::remove_dir_all(&project_path);
            let _ = std::fs::create_dir_all(&project_path);
        }
        Err(e) => {
            log::warn!("Failed to spawn git: {}", e);
        }
    }
}