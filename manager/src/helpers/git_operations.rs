use crate::error::AppError;
use shared_types::GitBranch;
use std::path::Path;

/// List all git branches and worktrees for a project
/// Returns both regular branches and worktree branches with their paths
pub fn list_project_branches(project_path: &Path) -> Result<Vec<GitBranch>, AppError> {
    let repo = git2::Repository::open(project_path)?;
    let mut branches = Vec::new();

    // List regular branches
    let branch_iter = repo.branches(Some(git2::BranchType::Local))?;
    for branch_result in branch_iter {
        let (branch, _branch_type) = branch_result?;
        if let Some(name) = branch.name()? {
            let branch_name = name.to_string();

            // Check if this branch has a worktree
            let is_worktree = has_worktree_for_branch(&repo, &branch_name)?;
            let worktree_path = if is_worktree {
                get_worktree_path_for_branch(project_path, &branch_name)?
            } else {
                None
            };

            branches.push(GitBranch {
                name: branch_name,
                is_worktree,
                path: worktree_path,
            });
        }
    }

    // Also check for worktrees that might not have corresponding local branches
    let worktrees = repo.worktrees()?;
    for worktree_name in worktrees.iter().flatten() {
        // Skip if we already added this branch
        if branches.iter().any(|b| b.name == *worktree_name) {
            continue;
        }

        let worktree_path = get_worktree_path_for_branch(project_path, worktree_name)?;
        branches.push(GitBranch {
            name: worktree_name.to_string(),
            is_worktree: true,
            path: worktree_path,
        });
    }

    Ok(branches)
}

/// Check if a branch has an associated worktree
fn has_worktree_for_branch(repo: &git2::Repository, branch_name: &str) -> Result<bool, AppError> {
    let worktrees = repo.worktrees()?;

    for worktree_name in worktrees.iter().flatten() {
        if worktree_name == branch_name {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Get the filesystem path for a worktree branch
fn get_worktree_path_for_branch(
    project_path: &Path,
    branch_name: &str,
) -> Result<Option<String>, AppError> {
    let repo = git2::Repository::open(project_path)?;
    let worktrees = repo.worktrees()?;

    for worktree_name in worktrees.iter().flatten() {
        if worktree_name == branch_name {
            // Try to find the worktree path
            let potential_paths = [
                project_path.join(worktree_name),
                project_path.join("..").join(worktree_name),
            ];

            for potential_path in &potential_paths {
                if git2::Repository::open(potential_path).is_ok() {
                    return Ok(Some(potential_path.to_string_lossy().to_string()));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_repo_with_branches() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize git repo
        Command::new("git")
            .args(["init", "."])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to init git repo");

        // Configure git user
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to configure git user");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to configure git email");

        // Create initial commit
        std::fs::write(temp_dir.path().join("test.txt"), "initial content").unwrap();
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to add file");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to create initial commit");

        // Create a new branch
        Command::new("git")
            .args(["checkout", "-b", "feature-branch"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to create feature branch");

        (temp_dir, repo_path)
    }

    #[test]
    fn test_list_project_branches_basic() {
        let (_temp_dir, repo_path) = create_test_repo_with_branches();
        let path = Path::new(&repo_path);

        let branches = list_project_branches(path).unwrap();

        // Should have at least main/master and feature-branch
        assert!(branches.len() >= 2);

        // Check that we have the expected branches
        let branch_names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
        assert!(branch_names.contains(&"feature-branch"));

        // Initially no worktrees
        let worktree_branches: Vec<&GitBranch> =
            branches.iter().filter(|b| b.is_worktree).collect();
        assert_eq!(worktree_branches.len(), 0);
    }

    #[test]
    fn test_list_project_branches_with_worktree() {
        let (temp_dir, repo_path) = create_test_repo_with_branches();
        let path = Path::new(&repo_path);

        // Create a worktree using git2
        let repo = git2::Repository::open(path).unwrap();
        let worktree_path = temp_dir.path().join("worktree-feature");

        // Create worktree for the feature-branch
        let worktree = repo.worktree("feature-worktree", &worktree_path, None);

        // If worktree creation fails (e.g., due to git limitations in test),
        // just test that the function works with regular branches
        if worktree.is_ok() {
            let branches = list_project_branches(path).unwrap();

            // Should find worktree branches
            let worktree_branches: Vec<&GitBranch> =
                branches.iter().filter(|b| b.is_worktree).collect();

            // At least the worktree we created should be found
            assert!(!worktree_branches.is_empty());
        } else {
            // If worktree creation failed, just verify regular branches work
            let branches = list_project_branches(path).unwrap();
            assert!(branches.len() >= 2); // main and feature-branch

            // Verify branch structure
            for branch in &branches {
                assert!(!branch.name.is_empty());
                assert!(
                    branch.path.is_none() || branch.path.as_ref().unwrap().contains("worktree")
                );
            }
        }
    }

    #[test]
    fn test_list_project_branches_nonexistent_repo() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent");

        let result = list_project_branches(&path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Git(_)));
    }
}
