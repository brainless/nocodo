use manager_models::{GrepMatch, GrepRequest, GrepResponse, ToolErrorResponse, ToolResponse};
use crate::tool_error::ToolError;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub async fn grep_search(
    base_path: &PathBuf,
    request: GrepRequest,
) -> Result<ToolResponse> {
    use regex::RegexBuilder;

    let search_path = if let Some(path) = &request.path {
        validate_and_resolve_path(base_path, path)?
    } else {
        base_path.clone()
    };

    if !search_path.exists() {
        return Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "grep".to_string(),
            error: "PathNotFound".to_string(),
            message: format!(
                "Search path does not exist: {}",
                request.path.unwrap_or_else(|| ".".to_string())
            ),
        }));
    }

    // Compile regex pattern
    let regex = RegexBuilder::new(&request.pattern)
        .case_insensitive(!request.case_sensitive.unwrap_or(false))
        .build()
        .map_err(|e| ToolError::InvalidPath(format!("Invalid regex pattern: {}", e)))?;

    // Compile include/exclude patterns (convert from glob to regex)
    let include_regex =
        if let Some(pattern) = &request.include_pattern {
            let regex_pattern = glob_to_regex(pattern);
            Some(RegexBuilder::new(&regex_pattern).build().map_err(|e| {
                ToolError::InvalidPath(format!("Invalid include pattern: {}", e))
            })?)
        } else {
            None
        };

    let exclude_regex =
        if let Some(pattern) = &request.exclude_pattern {
            let regex_pattern = glob_to_regex(pattern);
            Some(RegexBuilder::new(&regex_pattern).build().map_err(|e| {
                ToolError::InvalidPath(format!("Invalid exclude pattern: {}", e))
            })?)
        } else {
            None
        };

    let mut matches = Vec::new();
    let mut files_searched = 0;
    let max_results = request.max_results.unwrap_or(100) as usize;
    let max_files_searched = request.max_files_searched.unwrap_or(1000) as usize;

    // Use walkdir for recursive search if requested
    let recursive = request.recursive.unwrap_or(true);
    let walker = if recursive {
        WalkDir::new(&search_path)
    } else {
        WalkDir::new(&search_path).max_depth(1)
    };

    for entry in walker {
        let entry = entry.map_err(|e| ToolError::IoError(e.to_string()))?;

        // Skip directories
        if entry.file_type().is_dir() {
            continue;
        }

        // Check include/exclude patterns
        let file_path = entry.path();

        // Calculate relative path for display
        // When searching a single file, use the file name instead of empty string
        let relative_path = if search_path.is_file() {
            file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file_path.to_string_lossy().to_string())
        } else {
            file_path
                .strip_prefix(&search_path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string()
        };

        // Apply include filter
        if let Some(ref include_re) = include_regex {
            if !include_re.is_match(&relative_path) {
                continue;
            }
        }

        // Apply exclude filter
        if let Some(ref exclude_re) = exclude_regex {
            if exclude_re.is_match(&relative_path) {
                continue;
            }
        }

        // Skip files that don't match common patterns (like .gitignore)
        let file_name = entry.file_name().to_string_lossy();
        let file_path_str = relative_path.clone();

        // Skip common build artifacts and directories
        let skip_patterns = [
            "target",
            "node_modules",
            ".git",
            "dist",
            "build",
            "__pycache__",
            ".next",
            ".nuxt",
            ".vuepress",
            ".cache",
            ".parcel-cache",
            ".DS_Store",
            "Thumbs.db",
            "desktop.ini",
        ];

        let should_skip = file_name.starts_with('.')
            || skip_patterns.contains(&file_name.as_ref())
            || file_name.ends_with(".pyc")
            || file_name.ends_with(".pyo")
            || file_name == "Cargo.lock"
            || file_name == "package-lock.json"
            || file_name == "yarn.lock"
            || file_name == "pnpm-lock.yaml"
            || file_path_str.contains("/target/")
            || file_path_str.contains("/node_modules/")
            || file_path_str.contains("/.git/")
            || file_path_str.contains("/dist/")
            || file_path_str.contains("/build/")
            || file_path_str.contains("/__pycache__/");

        if should_skip {
            continue;
        }

        // Skip binary files by checking file extension and attempting to read as UTF-8
        let is_likely_binary = file_name.ends_with(".exe")
            || file_name.ends_with(".dll")
            || file_name.ends_with(".so")
            || file_name.ends_with(".dylib")
            || file_name.ends_with(".bin")
            || file_name.ends_with(".jpg")
            || file_name.ends_with(".jpeg")
            || file_name.ends_with(".png")
            || file_name.ends_with(".gif")
            || file_name.ends_with(".bmp")
            || file_name.ends_with(".tiff")
            || file_name.ends_with(".ico")
            || file_name.ends_with(".pdf")
            || file_name.ends_with(".zip")
            || file_name.ends_with(".tar")
            || file_name.ends_with(".gz")
            || file_name.ends_with(".bz2")
            || file_name.ends_with(".xz")
            || file_name.ends_with(".7z")
            || file_name.ends_with(".rar");

        if is_likely_binary {
            continue;
        }

        files_searched += 1;

        // Check if we've reached the max files searched limit
        if files_searched >= max_files_searched {
            break;
        }

        // Search file content
        let content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(_) => continue, // Skip files we can't read
        };

        // Search for pattern in content
        for (line_num, line) in content.lines().enumerate() {
            if matches.len() >= max_results {
                break;
            }

            // Find all matches in this line
            for mat in regex.find_iter(line) {
                if matches.len() >= max_results {
                    break;
                }

                let matched_text = mat.as_str().to_string();

                let grep_match = GrepMatch {
                    file_path: relative_path.clone(),
                    line_number: if request.include_line_numbers.unwrap_or(true) {
                        Some((line_num + 1) as u32)
                    } else {
                        None
                    },
                    line_content: line.to_string(),
                    match_start: mat.start() as u32,
                    match_end: mat.end() as u32,
                    matched_text,
                };

                matches.push(grep_match);
            }
        }

        // Stop if we've reached the max results limit
        if matches.len() >= max_results {
            break;
        }
    }

    let mut total_matches = matches.len() as u32;
    let mut truncated = matches.len() >= max_results;

    // Check response size and truncate if necessary (limit to ~100KB)
    const MAX_RESPONSE_SIZE: usize = 100 * 1024; // 100KB
    let response_size_estimate = matches
        .iter()
        .map(|m| m.file_path.len() + m.line_content.len() + m.matched_text.len() + 100) // rough estimate
        .sum::<usize>();

    if response_size_estimate > MAX_RESPONSE_SIZE {
        // Truncate matches to fit within size limit
        let mut truncated_matches = Vec::new();
        let mut current_size = 0;

        for match_item in matches {
            let item_size = match_item.file_path.len()
                + match_item.line_content.len()
                + match_item.matched_text.len()
                + 100;
            if current_size + item_size > MAX_RESPONSE_SIZE {
                truncated = true;
                break;
            }
            current_size += item_size;
            truncated_matches.push(match_item);
        }

        matches = truncated_matches;
        total_matches = matches.len() as u32;
    }

    Ok(ToolResponse::Grep(GrepResponse {
        pattern: request.pattern,
        matches,
        total_matches,
        files_searched: files_searched as u32,
        truncated,
    }))
}

/// Convert glob pattern to regex pattern
/// Examples:
/// - *.rs -> .*\.rs$
/// - *.py -> .*\.py$
/// - test*.txt -> ^test.*\.txt$
/// - **/*.rs -> .*/.*\.rs$ (for nested paths)
fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::new();
    let mut chars = glob.chars().peekable();

    // Add start anchor unless pattern starts with ** or *
    if !glob.starts_with("**") && !glob.starts_with('*') {
        regex.push('^');
    }

    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                // Check for ** pattern (match any directory depth)
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                    regex.push_str(".*");
                } else {
                    // Single * matches any characters except path separator
                    regex.push_str("[^/]*");
                }
            }
            '?' => {
                // ? matches any single character except path separator
                regex.push_str("[^/]");
            }
            '.' | '+' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' => {
                // Escape regex special characters
                regex.push('\\');
                regex.push(ch);
            }
            _ => {
                regex.push(ch);
            }
        }
    }

    // Add end anchor if pattern doesn't contain directory separators or wildcards at the end
    if !regex.ends_with(".*") {
        regex.push('$');
    }

    regex
}

/// Validate and resolve a path relative to the base path
fn validate_and_resolve_path(base_path: &PathBuf, path: &str) -> Result<PathBuf> {
    let input_path = Path::new(path);

    // Normalize the input path to handle . and .. components
    let normalized_input = normalize_path(input_path)?;

    // Handle absolute paths
    if normalized_input.is_absolute() {
        // If the absolute path equals our base path, allow it
        let canonical_input = match normalized_input.canonicalize() {
            Ok(path) => path,
            Err(_) => normalized_input.to_path_buf(), // Fallback if it doesn't exist yet
        };

        let canonical_base = match base_path.canonicalize() {
            Ok(path) => path,
            Err(_) => base_path.clone(),
        };

        // Security check: ensure the path is within or equals the base directory
        if canonical_input == canonical_base || canonical_input.starts_with(&canonical_base) {
            return Ok(canonical_input);
        } else {
            return Err(ToolError::InvalidPath(format!(
                "Absolute path '{}' is outside the allowed directory '{}'",
                path,
                base_path.display()
            ))
            .into());
        }
    }

    // Handle relative paths
    let target_path = if normalized_input == Path::new(".") {
        base_path.clone()
    } else {
        base_path.join(&normalized_input)
    };

    // Canonicalize the path to resolve any remaining relative components
    let canonical_path = match target_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            // If file doesn't exist, try to canonicalize parent directory
            // and reconstruct the path to handle symlink issues on macOS
            if let Some(parent) = target_path.parent() {
                match parent.canonicalize() {
                    Ok(canonical_parent) => {
                        if let Some(filename) = target_path.file_name() {
                            canonical_parent.join(filename)
                        } else {
                            target_path
                        }
                    }
                    Err(_) => target_path,
                }
            } else {
                target_path
            }
        }
    };

    // Also canonicalize the base path for comparison (handles symlinks on macOS)
    let canonical_base = match base_path.canonicalize() {
        Ok(path) => path,
        Err(_) => base_path.clone(), // Fallback to non-canonical base path
    };

    // Security check: ensure the path is within the base directory
    if !canonical_path.starts_with(&canonical_base) {
        return Err(ToolError::InvalidPath(format!(
            "Path '{}' resolves to location outside the allowed directory",
            path
        ))
        .into());
    }

    Ok(canonical_path)
}

/// Normalize a path by resolving . and .. components while preventing directory traversal
fn normalize_path(path: &Path) -> Result<PathBuf> {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                // For absolute paths, keep the prefix/root
                components.push(component);
            }
            std::path::Component::CurDir => {
                // Skip current directory components
                continue;
            }
            std::path::Component::ParentDir => {
                // Prevent directory traversal attacks
                if components.is_empty()
                    || matches!(components.last(), Some(std::path::Component::ParentDir))
                {
                    return Err(ToolError::InvalidPath(format!(
                        "Invalid path '{}': contains directory traversal",
                        path.display()
                    ))
                    .into());
                }
                // Remove the last component (go up one level)
                components.pop();
            }
            std::path::Component::Normal(_name) => {
                components.push(component);
            }
        }
    }

    // Reconstruct the path from components
    let mut result = PathBuf::new();
    for component in components {
        result.push(component);
    }

    Ok(result)
}