use std::path::Path;

const TEMPLATE_REPO_URL: &str = "https://github.com/brainless/rustysolid.git";
const STACK_NOTES_SEED_FILE: &str = "stack_notes_seed.sql";
const DEFAULT_DB_KIND: &str = "sqlite";

/// After cloning the template, select the correct database variant files.
/// Copies `backend/src/db.rs.{kind}` → `backend/src/db.rs`
/// Copies `backend/Cargo.toml.{kind}` → `backend/Cargo.toml`
/// Removes the suffix files after selection.
fn select_project_db(project_path: &str, db_kind: &str) {
    let db_src = Path::new(project_path)
        .join("backend")
        .join("src")
        .join(format!("db.rs.{db_kind}"));
    let db_dst = Path::new(project_path)
        .join("backend")
        .join("src")
        .join("db.rs");
    let cargo_src = Path::new(project_path)
        .join("backend")
        .join(format!("Cargo.toml.{db_kind}"));
    let cargo_dst = Path::new(project_path).join("backend").join("Cargo.toml");

    if db_src.exists() {
        match std::fs::copy(&db_src, &db_dst) {
            Ok(_) => {
                log::info!("[select_project_db] Selected {} db.rs variant", db_kind);
                let _ = std::fs::remove_file(&db_src);
            }
            Err(e) => {
                log::warn!(
                    "[select_project_db] Failed to copy db.rs.{}: {}",
                    db_kind,
                    e
                );
            }
        }
    } else {
        log::warn!(
            "[select_project_db] db.rs.{} not found at {}",
            db_kind,
            db_src.display()
        );
    }

    if cargo_src.exists() {
        match std::fs::copy(&cargo_src, &cargo_dst) {
            Ok(_) => {
                log::info!(
                    "[select_project_db] Selected {} Cargo.toml variant",
                    db_kind
                );
                let _ = std::fs::remove_file(&cargo_src);
            }
            Err(e) => {
                log::warn!(
                    "[select_project_db] Failed to copy Cargo.toml.{}: {}",
                    db_kind,
                    e
                );
            }
        }
    } else {
        log::warn!(
            "[select_project_db] Cargo.toml.{} not found at {}",
            db_kind,
            cargo_src.display()
        );
    }
}

/// Read stack_notes_seed.sql from the cloned project and insert rows for project_id.
/// Each statement in the file uses ?1 as the project_id placeholder.
fn seed_stack_notes(db_url: &str, project_id: i64, project_path: &str) {
    let seed_path = Path::new(project_path).join(STACK_NOTES_SEED_FILE);
    let sql = match std::fs::read_to_string(&seed_path) {
        Ok(s) => s,
        Err(e) => {
            log::warn!(
                "[seed_stack_notes] Could not read {}: {}",
                seed_path.display(),
                e
            );
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
    for stmt in split_sql_statements(&sql) {
        let stmt = stmt.trim();
        if stmt.is_empty() || stmt.starts_with("--") {
            continue;
        }
        match conn.execute(stmt, rusqlite::params![project_id]) {
            Ok(_) => count += 1,
            Err(e) => log::warn!("[seed_stack_notes] Statement failed: {} — {}", stmt, e),
        }
    }
    log::info!(
        "[seed_stack_notes] Seeded {} stack notes for project {}",
        count,
        project_id
    );
}

/// Split SQL on semicolons while respecting single-quoted string literals.
fn split_sql_statements(sql: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut start = 0;
    let mut in_quote = false;
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\'' if !in_quote => in_quote = true,
            '\'' if in_quote => {
                // Handle escaped quotes ('')
                if i + 1 < chars.len() && chars[i + 1] == '\'' {
                    i += 1;
                } else {
                    in_quote = false;
                }
            }
            ';' if !in_quote => {
                statements.push(&sql[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < sql.len() {
        statements.push(&sql[start..]);
    }
    statements
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
            select_project_db(&project_path, DEFAULT_DB_KIND);
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
