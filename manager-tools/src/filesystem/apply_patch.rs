use super::path_utils::{validate_and_resolve_path};
use anyhow::Result;
use manager_models::{
    ApplyPatchFileChange, ApplyPatchRequest, ApplyPatchResponse, ToolErrorResponse, ToolResponse,
};
use std::fs;
use std::path::PathBuf;

pub async fn apply_patch(base_path: &PathBuf, request: ApplyPatchRequest) -> Result<ToolResponse> {
    use codex_apply_patch::{parse_patch, Hunk};

    // Parse the patch
    let parsed = match parse_patch(&request.patch) {
        Ok(parsed) => parsed,
        Err(e) => {
            return Ok(ToolResponse::Error(ToolErrorResponse {
                tool: "apply_patch".to_string(),
                error: "ParseError".to_string(),
                message: format!("Failed to parse patch: {}", e),
            }));
        }
    };

    // Change to base directory before applying patch
    let original_dir = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?;

    std::env::set_current_dir(base_path)
        .map_err(|e| anyhow::anyhow!("Failed to change to base directory: {}", e))?;

    let mut files_changed = Vec::new();
    let mut total_additions = 0;
    let mut total_deletions = 0;
    let mut errors = Vec::new();

    // Process each hunk
    for hunk in &parsed.hunks {
        match hunk {
            Hunk::AddFile { path, contents } => {
                // Validate path
                let path_str = path.to_string_lossy().to_string();
                if let Err(e) = validate_and_resolve_path(base_path, &path_str) {
                    errors.push(format!("Invalid path '{}': {}", path_str, e));
                    continue;
                }

                // Create parent directories if needed
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        if let Err(e) = fs::create_dir_all(parent) {
                            errors.push(format!(
                                "Failed to create parent directory for '{}': {}",
                                path_str, e
                            ));
                            continue;
                        }
                    }
                }

                // Write the new file
                if let Err(e) = fs::write(path, contents) {
                    errors.push(format!("Failed to create file '{}': {}", path_str, e));
                    continue;
                }

                let line_count = contents.lines().count();
                total_additions += line_count;

                files_changed.push(ApplyPatchFileChange {
                    path: path_str,
                    operation: "add".to_string(),
                    new_path: None,
                    unified_diff: None,
                });
            }
            Hunk::DeleteFile { path } => {
                let path_str = path.to_string_lossy().to_string();
                if let Err(e) = validate_and_resolve_path(base_path, &path_str) {
                    errors.push(format!("Invalid path '{}': {}", path_str, e));
                    continue;
                }

                // Read the file first to count deletions
                if let Ok(content) = fs::read_to_string(path) {
                    total_deletions += content.lines().count();
                }

                // Delete the file
                if let Err(e) = fs::remove_file(path) {
                    errors.push(format!("Failed to delete file '{}': {}", path_str, e));
                    continue;
                }

                files_changed.push(ApplyPatchFileChange {
                    path: path_str,
                    operation: "delete".to_string(),
                    new_path: None,
                    unified_diff: None,
                });
            }
            Hunk::UpdateFile {
                path,
                move_path,
                chunks,
            } => {
                let path_str = path.to_string_lossy().to_string();
                if let Err(e) = validate_and_resolve_path(base_path, &path_str) {
                    errors.push(format!("Invalid path '{}': {}", path_str, e));
                    continue;
                }

                // Read original content
                let original_content = match fs::read_to_string(path) {
                    Ok(content) => content,
                    Err(e) => {
                        errors.push(format!("Failed to read file '{}': {}", path_str, e));
                        continue;
                    }
                };

                // Apply chunks using codex-apply-patch's logic
                let mut original_lines: Vec<String> =
                    original_content.split('\n').map(String::from).collect();
                if original_lines.last().is_some_and(String::is_empty) {
                    original_lines.pop();
                }

                // Apply each chunk to the file
                let mut modified_content = original_content.clone();

                for chunk in chunks {
                    total_deletions += chunk.old_lines.len();
                    total_additions += chunk.new_lines.len();

                    // Find and replace the old_lines with new_lines
                    let old_text = chunk.old_lines.join("\n");
                    let new_text = chunk.new_lines.join("\n");

                    // Try to find the exact match first
                    if let Some(pos) = modified_content.find(&old_text) {
                        // Replace the found text
                        modified_content.replace_range(pos..pos + old_text.len(), &new_text);
                    } else {
                        // If exact match fails, try with context
                        if let Some(ref context) = chunk.change_context {
                            // Find the context line first
                            if let Some(context_pos) = modified_content.find(context) {
                                // Search for old_lines after the context
                                let search_start = context_pos + context.len();
                                if let Some(relative_pos) =
                                    modified_content[search_start..].find(&old_text)
                                {
                                    let absolute_pos = search_start + relative_pos;
                                    modified_content.replace_range(
                                        absolute_pos..absolute_pos + old_text.len(),
                                        &new_text,
                                    );
                                } else {
                                    errors.push(format!(
                                        "Could not find old lines in '{}' after context '{}'",
                                        path_str, context
                                    ));
                                    continue;
                                }
                            } else {
                                errors.push(format!(
                                    "Could not find context '{}' in '{}'",
                                    context, path_str
                                ));
                                continue;
                            }
                        } else {
                            errors.push(format!(
                                "Could not find old lines in '{}' and no context provided",
                                path_str
                            ));
                            continue;
                        }
                    }
                }

                // Write the modified content back to the file
                if let Err(e) = fs::write(path, modified_content) {
                    errors.push(format!(
                        "Failed to write modified file '{}': {}",
                        path_str, e
                    ));
                    continue;
                }

                let operation = if move_path.is_some() {
                    "move"
                } else {
                    "update"
                };

                files_changed.push(ApplyPatchFileChange {
                    path: path_str,
                    operation: operation.to_string(),
                    new_path: move_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                    unified_diff: None,
                });
            }
        }
    }

    // Restore original directory
    let _ = std::env::set_current_dir(original_dir);

    // Determine success
    let success = errors.is_empty();
    let message = if success {
        format!(
            "Successfully applied patch: {} file(s) changed, {} additions(+), {} deletions(-)",
            files_changed.len(),
            total_additions,
            total_deletions
        )
    } else {
        format!(
            "Patch partially applied with {} error(s): {}",
            errors.len(),
            errors.join("; ")
        )
    };

    Ok(ToolResponse::ApplyPatch(ApplyPatchResponse {
        success,
        files_changed,
        total_additions,
        total_deletions,
        message,
    }))
}


