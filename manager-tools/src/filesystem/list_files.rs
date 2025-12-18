use super::path_utils::validate_and_resolve_path;
use anyhow::Result;
use manager_models::{
    FileInfo, FileType, ListFilesRequest, ListFilesResponse, ToolErrorResponse, ToolResponse,
};
use std::fs;
use std::path::Path;

pub async fn list_files(base_path: &Path, request: ListFilesRequest) -> Result<ToolResponse> {
    let target_path = validate_and_resolve_path(base_path, &request.path)?;

    if !target_path.exists() {
        return Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "list_files".to_string(),
            error: "FileNotFound".to_string(),
            message: format!("Path does not exist: {}", request.path),
        }));
    }

    if !target_path.is_dir() {
        return Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "list_files".to_string(),
            error: "InvalidPath".to_string(),
            message: format!("Path is not a directory: {}", request.path),
        }));
    }

    let recursive = request.recursive.unwrap_or(false);
    let include_hidden = request.include_hidden.unwrap_or(false);
    let max_files = request.max_files.unwrap_or(100) as usize;

    // Collect all files with breadth-first traversal
    let mut all_files = Vec::new();
    let mut queue = vec![target_path.clone()];
    let mut visited = std::collections::HashSet::new();

    while !queue.is_empty() && all_files.len() < max_files {
        let current_dir = queue.remove(0);

        if visited.contains(&current_dir) {
            continue;
        }
        visited.insert(current_dir.clone());

        let entries = match fs::read_dir(&current_dir) {
            Ok(entries) => entries,
            Err(_) => continue, // Skip directories we can't read
        };

        let mut subdirs = Vec::new();

        for entry in entries {
            if all_files.len() >= max_files {
                break;
            }

            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let path = entry.path();

            // Skip hidden files/directories if not requested
            if !include_hidden {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with('.') {
                    continue;
                }
            }

            let file_info = match create_file_info(&path, &target_path) {
                Ok(info) => info,
                Err(_) => continue,
            };

            if matches!(file_info.file_type, FileType::Directory) {
                subdirs.push(path);
            }

            all_files.push(file_info);
        }

        // Add subdirectories to queue for breadth-first traversal
        if recursive {
            queue.extend(subdirs);
        }
    }

    // Sort files: directories first, then by name (case-insensitive)
    all_files.sort_by(|a, b| {
        match (&a.file_type, &b.file_type) {
            (FileType::Directory, FileType::File) => std::cmp::Ordering::Less,
            (FileType::File, FileType::Directory) => std::cmp::Ordering::Greater,
            _ => {
                // Both are same type, sort by name case-insensitively
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then_with(|| a.name.cmp(&b.name)) // Stable sort for same lowercase names
            }
        }
    });

    // Generate tree representation
    let tree_output = format_as_tree(&all_files, &target_path);

    let total_files = all_files.len() as u32;
    let truncated = all_files.len() >= max_files;

    Ok(ToolResponse::ListFiles(ListFilesResponse {
        current_path: request.path,
        files: tree_output,
        total_files,
        truncated,
        limit: max_files as u32,
    }))
}

/// Create FileInfo from a path
fn create_file_info(path: &Path, base_path: &Path) -> Result<FileInfo> {
    let metadata = fs::metadata(path)?;

    let relative_path = path
        .strip_prefix(base_path)
        .map_err(|_| anyhow::anyhow!("Cannot compute relative path for {:?}", path))?;

    let relative_path_str = relative_path.to_string_lossy().to_string();
    let absolute_path_str = path.to_string_lossy().to_string();

    // Check if file is ignored by .gitignore
    let ignored = is_ignored_by_gitignore(path)?;
    let is_directory = metadata.is_dir();

    Ok(FileInfo {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        path: relative_path_str,
        absolute: absolute_path_str,
        file_type: if is_directory {
            FileType::Directory
        } else {
            FileType::File
        },
        ignored,
        is_directory,
        size: if is_directory {
            None
        } else {
            metadata.len().into()
        },
        modified_at: if is_directory {
            None
        } else {
            metadata.modified().ok().map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .to_string()
            })
        },
    })
}

/// Format files as a tree structure
fn format_as_tree(files: &[FileInfo], base_path: &Path) -> String {
    let mut output = String::new();

    // Add root directory name
    let root_name = base_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    output.push_str(&root_name);
    output.push('\n');

    // Group files by their directory depth and parent
    let mut file_tree: std::collections::BTreeMap<String, Vec<&FileInfo>> =
        std::collections::BTreeMap::new();

    for file in files.iter() {
        let path_parts: Vec<&str> = file.path.split('/').collect();
        let depth = path_parts.len().saturating_sub(1);

        // Create a key for the parent directory at this depth
        let parent_key = if depth == 0 {
            "".to_string()
        } else {
            path_parts[..depth].join("/")
        };

        file_tree.entry(parent_key).or_default().push(file);
    }

    // Recursive function to build tree
    fn build_tree_level(
        output: &mut String,
        tree: &std::collections::BTreeMap<String, Vec<&FileInfo>>,
        current_path: &str,
        prefix: &str,
    ) {
        let files = match tree.get(current_path) {
            Some(files) => files,
            None => return,
        };

        for file in files.iter() {
            output.push_str(&format!("{}  {}", prefix, file.name));

            if file.ignored {
                output.push_str(" (ignored)");
            }

            output.push('\n');

            // If it's a directory, recurse
            if matches!(file.file_type, FileType::Directory) {
                let child_path = if current_path.is_empty() {
                    file.name.clone()
                } else {
                    format!("{}/{}", current_path, file.name)
                };
                build_tree_level(output, tree, &child_path, &format!("{}  ", prefix));
            }
        }
    }

    build_tree_level(&mut output, &file_tree, "", "");
    output
}

/// Check if a file is ignored by .gitignore
fn is_ignored_by_gitignore(file_path: &Path) -> Result<bool> {
    // For now, implement basic ignore patterns
    // TODO: Implement full .gitignore parsing
    let file_name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Common ignore patterns
    let ignore_patterns = [
        "node_modules",
        ".git",
        "dist",
        "build",
        ".next",
        "__pycache__",
        "*.pyc",
        ".DS_Store",
        "target", // Rust build directory
        "Cargo.lock",
    ];

    // Check if file name matches any ignore pattern
    for pattern in &ignore_patterns {
        if file_name == *pattern || file_name.starts_with(&format!("{}.", pattern)) {
            return Ok(true);
        }
    }

    // Check if any component in the path matches ignore patterns
    for component in file_path.components() {
        let comp_str = component.as_os_str().to_string_lossy();
        for pattern in &ignore_patterns {
            if comp_str == *pattern {
                return Ok(true);
            }
        }
    }

    Ok(false)
}
