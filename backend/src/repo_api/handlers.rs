use std::path::Path;

const TEMPLATE_REPO_URL: &str = "https://github.com/brainless/rustysolid.git";
const STACK_NOTES_SEED_FILE: &str = "stack_notes_seed.sql";

/// Read stack_notes_seed.sql from the cloned project and insert rows for project_id.
/// Each statement in the file uses ?1 as the project_id placeholder.
fn seed_stack_notes(db_url: &str, project_id: i64, project_path: &str) {
    let seed_path = Path::new(project_path).join(STACK_NOTES_SEED_FILE);
    let sql = match std::fs::read_to_string(&seed_path) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("[seed_stack_notes] Could not read {}: {}", seed_path.display(), e);
            return;
        }
    };
    let conn = match rusqlite::Connection::open(db_url) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("[seed_stack_notes] Could not open DB: {}", e);
            return;
        }
    };
    let mut count = 0usize;
    for raw in sql.split(';') {
        let stmt = raw.trim();
        if stmt.is_empty() || stmt.starts_with("--") {
            continue;
        }
        match conn.execute(stmt, rusqlite::params![project_id]) {
            Ok(_) => count += 1,
            Err(e) => log::warn!("[seed_stack_notes] Statement failed: {} — {}", stmt, e),
        }
    }
    log::info!("[seed_stack_notes] Seeded {} stack notes for project {}", count, project_id);
}

/// Clone template repo into project_path, then seed stack notes.
/// Skips clone silently if directory already has a git repo.
pub async fn clone_template_repo(project_path: String, db_url: String, project_id: i64) {
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
            seed_stack_notes(&db_url, project_id, &project_path);
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
