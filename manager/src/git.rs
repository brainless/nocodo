use std::path::Path;
use crate::error::AppError;

/// Get the working directory path for a given git branch.
/// If the branch has a worktree, returns the worktree path.
/// If no worktree exists for the branch, returns an error.
#[allow(dead_code)]
pub fn get_working_directory_for_branch(project_path: &Path, branch_name: &str) -> Result<String, AppError> {
    let repo = git2::Repository::open(project_path)?;
    
    // Check if this is a worktree branch by looking at git worktree list
    let worktrees = repo.worktrees()?;
    
    for worktree_name in worktrees.iter().flatten() {
        // Try to find the worktree path by checking common locations
        // Worktrees are typically in subdirectories of the main repository
        let potential_paths = [
            project_path.join(worktree_name),
            project_path.join("..").join(worktree_name),
        ];
        
        for potential_path in &potential_paths {
            if let Ok(worktree_repo) = git2::Repository::open(potential_path) {
                if let Ok(head) = worktree_repo.head() {
                    if let Some(head_ref) = head.name() {
                        // Extract branch name from reference (e.g., "refs/heads/feature-branch" -> "feature-branch")
                        if let Some(branch) = head_ref.strip_prefix("refs/heads/") {
                            if branch == branch_name {
                                return Ok(potential_path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    Err(AppError::InvalidRequest(format!(
        "No worktree found for branch '{}'", 
        branch_name
    )))
}