use std::path::Path;
use crate::error::AppError;
use git2::{Repository, BranchType};

/// List local branches that have associated worktrees
pub fn list_local_branches_with_worktrees(project_path: &Path) -> Result<Vec<String>, AppError> {
    // Open the git repository
    let repo = Repository::open(project_path).map_err(|e| {
        match e.class() {
            git2::ErrorClass::Config => AppError::InvalidRequest(
                "Not a git repository".to_string(),
            ),
            _ => AppError::Internal(format!("Failed to open git repository: {e}")),
        }
    })?;

    tracing::debug!("Getting worktree branches for path: {:?}", project_path);

    // Get list of worktrees
    let worktrees = repo.worktrees().map_err(|e| {
        AppError::Internal(format!("Failed to get worktrees: {e}"))
    })?;

    let mut worktree_branches = std::collections::HashSet::new();

    // Extract branch names from worktrees
    for worktree_name in worktrees.iter().flatten() {
        if let Ok(worktree) = repo.find_worktree(worktree_name) {
            // Get the worktree path and try to read its HEAD
            let worktree_path = worktree.path();
            let git_dir = if worktree_path.join(".git").is_file() {
                // This is a worktree with a .git file pointing to the main repo
                std::fs::read_to_string(worktree_path.join(".git")).ok()
                    .and_then(|content| {
                        content.lines()
                            .find(|line| line.starts_with("gitdir:"))
                            .and_then(|line| line.strip_prefix("gitdir: "))
                            .map(|s| s.trim().to_owned())
                    })
                    .map(|s| std::path::PathBuf::from(s))
            } else {
                // This might be the main worktree or a separate git dir
                Some(worktree_path.join(".git"))
            };

            if let Some(git_dir) = git_dir {
                // Try to read the HEAD reference from the worktree
                let head_path = git_dir.join("HEAD");
                if head_path.exists() {
                    if let Ok(head_content) = std::fs::read_to_string(&head_path) {
                        for line in head_content.lines() {
                            if line.starts_with("ref: refs/heads/") {
                                let branch_name = line.strip_prefix("ref: refs/heads/").unwrap_or("").to_owned();
                                if !branch_name.is_empty() {
                                    tracing::debug!("Found worktree branch: {}", branch_name);
                                    worktree_branches.insert(branch_name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }



    tracing::debug!("Worktree branches set: {:?}", worktree_branches);

    // Get all local branches and filter by those with worktrees
    let mut result = Vec::new();
    
    if let Ok(branches) = repo.branches(Some(BranchType::Local)) {
        for branch_result in branches {
            if let Ok((branch, _branch_type)) = branch_result {
                if let Some(branch_name) = branch.name()? {
                    if worktree_branches.contains(branch_name) {
                        tracing::debug!("Including branch: {}", branch_name);
                        result.push(branch_name.to_string());
                    }
                }
            }
        }
    }

    tracing::debug!("Final result: {:?}", result);
    Ok(result)
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_list_local_branches_with_worktrees_non_git_repo() {
        let temp_dir = PathBuf::from("/tmp/nonexistent");
        let result = list_local_branches_with_worktrees(&temp_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_local_branches_with_worktrees_empty_repo() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(&temp_dir)?;
        
        // Create an initial commit to have a proper repository
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let sig = repo.signature()?;
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;
        
        let result = list_local_branches_with_worktrees(temp_dir.path())?;
        assert!(result.is_empty()); // No worktrees in a fresh repo
        
        Ok(())
    }
}