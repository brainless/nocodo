use std::path::Path;
use crate::error::AppError;
use git2::Repository;



/// Get the working directory path for a specific git branch
/// Returns the worktree path if the branch is in a worktree, or the project path for the main branch
pub fn get_working_directory_for_branch(project_path: &Path, branch_name: &str) -> Result<String, AppError> {
    let repo = Repository::open(project_path).map_err(|e| {
        match e.class() {
            git2::ErrorClass::Config => AppError::InvalidRequest(
                "Not a git repository".to_string(),
            ),
            _ => AppError::Internal(format!("Failed to open git repository: {e}")),
        }
    })?;

    tracing::debug!("Looking for worktree path for branch: {}", branch_name);

    // Get list of worktrees
    let worktrees = repo.worktrees().map_err(|e| {
        AppError::Internal(format!("Failed to get worktrees: {e}"))
    })?;

    // Search through worktrees to find the one with this branch
    for worktree_name in worktrees.iter().flatten() {
        if let Ok(worktree) = repo.find_worktree(worktree_name) {
            let worktree_path = worktree.path();

            // Get the git directory for this worktree
            let git_dir = if worktree_path.join(".git").is_file() {
                std::fs::read_to_string(worktree_path.join(".git")).ok()
                    .and_then(|content| {
                        content.lines()
                            .find(|line| line.starts_with("gitdir:"))
                            .and_then(|line| line.strip_prefix("gitdir: "))
                            .map(|s| s.trim().to_owned())
                    })
                    .map(|s| std::path::PathBuf::from(s))
            } else {
                Some(worktree_path.join(".git"))
            };

            if let Some(git_dir) = git_dir {
                // Read the HEAD reference from the worktree
                let head_path = git_dir.join("HEAD");
                if head_path.exists() {
                    if let Ok(head_content) = std::fs::read_to_string(&head_path) {
                        for line in head_content.lines() {
                            if line.starts_with("ref: refs/heads/") {
                                let found_branch = line.strip_prefix("ref: refs/heads/").unwrap_or("");
                                if found_branch == branch_name {
                                    // Found the worktree for this branch
                                    let full_path = worktree_path.to_str()
                                        .ok_or_else(|| AppError::Internal("Invalid UTF-8 in path".to_string()))?
                                        .to_string();
                                    tracing::info!("Found worktree for branch '{}': {}", branch_name, full_path);
                                    return Ok(full_path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If we didn't find a worktree, assume it's the main branch and return project path
    let project_path_str = project_path.to_str()
        .ok_or_else(|| AppError::Internal("Invalid UTF-8 in project path".to_string()))?
        .to_string();

    tracing::info!("Branch '{}' not found in worktrees, using project path: {}", branch_name, project_path_str);
    Ok(project_path_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    
}