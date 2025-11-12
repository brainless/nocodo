use std::path::Path;
use std::process::Command;
use crate::error::AppError;

/// List local branches that have associated worktrees
pub fn list_local_branches_with_worktrees(project_path: &Path) -> Result<Vec<String>, AppError> {
    // Check if the directory is a git repository
    let git_dir = project_path.join(".git");
    if !git_dir.exists() {
        return Err(AppError::InvalidRequest(
            "Not a git repository".to_string(),
        ));
    }

    // Get list of worktrees
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(project_path)
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run git worktree list: {e}")))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("Git worktree list failed: {error}")));
    }

    let worktree_output = String::from_utf8_lossy(&output.stdout);
    let mut worktree_branches = std::collections::HashSet::new();

    // Parse worktree output to extract branch names
    let mut lines = worktree_output.lines();
    while let Some(line) = lines.next() {
        if line.starts_with("worktree ") {
            // Next line should be the branch reference
            if let Some(branch_line) = lines.next() {
                if branch_line.starts_with("branch ") {
                    let branch_ref = branch_line.strip_prefix("branch ").unwrap_or("");
                    // Convert refs/heads/branch-name to branch-name
                    if let Some(branch_name) = branch_ref.strip_prefix("refs/heads/") {
                        worktree_branches.insert(branch_name.to_string());
                    }
                }
            }
        }
    }

    // Get all local branches
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .current_dir(project_path)
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run git branch: {e}")))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("Git branch failed: {error}")));
    }

    let branches_output = String::from_utf8_lossy(&output.stdout);
    let mut result = Vec::new();

    for branch in branches_output.lines() {
        let branch = branch.trim();
        if !branch.is_empty() && worktree_branches.contains(branch) {
            result.push(branch.to_string());
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_list_local_branches_with_worktrees_non_git_repo() {
        let temp_dir = PathBuf::from("/tmp/nonexistent");
        let result = list_local_branches_with_worktrees(&temp_dir);
        assert!(result.is_err());
    }
}