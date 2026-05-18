use std::path::{Component, Path, PathBuf};

/// Resolve a path given by the LLM (relative to project root) into an absolute PathBuf.
/// Returns `Err` with an error message string if the path is unsafe (absolute or traversal).
pub fn resolve_relative_path(project_path: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let rel = relative_path.trim();
    if rel.is_empty() || rel == "." {
        return Ok(project_path.to_path_buf());
    }
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err(
            "Error: absolute paths are not allowed. Use a path relative to project root."
                .to_string(),
        );
    }
    if rel_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(
            "Error: path traversal is not allowed. Use a path relative to project root."
                .to_string(),
        );
    }
    Ok(project_path.join(rel_path))
}

/// List the files and directories at `relative_path` under `project_path`.
/// Returns a formatted string listing directories (suffixed with `/`) then files.
pub fn list_files(project_path: &Path, relative_path: &str) -> String {
    let target = match resolve_relative_path(project_path, relative_path) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let entries = match std::fs::read_dir(&target) {
        Ok(rd) => rd,
        Err(e) => return format!("Error reading directory: {}", e),
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if let Ok(ft) = entry.file_type() {
            if ft.is_dir() {
                dirs.push(format!("{}/", name));
            } else {
                files.push(name);
            }
        }
    }

    dirs.sort();
    files.sort();

    let mut result = String::new();
    if !dirs.is_empty() {
        result.push_str("Directories:\n");
        for d in &dirs {
            result.push_str(&format!("  {}\n", d));
        }
    }
    if !files.is_empty() {
        result.push_str("Files:\n");
        for f in &files {
            result.push_str(&format!("  {}\n", f));
        }
    }
    if result.is_empty() {
        result = "(empty directory)\n".to_string();
    }
    result
}

/// Read a file at `relative_path` under `project_path`.
/// Returns file contents (capped at 500 lines) or an error string.
pub fn read_file(project_path: &Path, relative_path: &str) -> String {
    let target = match resolve_relative_path(project_path, relative_path) {
        Ok(p) => p,
        Err(e) => return e,
    };
    match std::fs::read_to_string(&target) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            if lines.len() > 500 {
                format!(
                    "(File has {} lines, showing first 500)\n{}",
                    lines.len(),
                    lines[..500].join("\n")
                )
            } else {
                content
            }
        }
        Err(e) => format!("Error reading file: {}", e),
    }
}
