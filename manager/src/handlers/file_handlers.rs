use super::main_handlers::AppState;
use crate::error::AppError;
use crate::models::{
    FileContentResponse, FileCreateRequest, FileInfo, FileListRequest, FileListResponse,
    FileResponse, FileType, FileUpdateRequest,
};
use actix_web::{web, HttpResponse, Result};
use std::path::Path;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[allow(dead_code)]
pub async fn list_files(
    data: web::Data<AppState>,
    query: web::Query<FileListRequest>,
) -> Result<HttpResponse, AppError> {
    let request = query.into_inner();

    tracing::info!(
        "list_files called with project_id: {:?}, path: {:?}, git_branch: {:?}",
        request.project_id,
        request.path,
        request.git_branch
    );

    // Get the project to determine the base path
    let project = if let Some(project_id) = &request.project_id {
        data.database.get_project_by_id(*project_id)?
    } else {
        return Err(AppError::InvalidRequest(
            "project_id is required".to_string(),
        ));
    };

    // Determine the base path - use worktree path if git_branch is specified
    let project_path = if let Some(ref git_branch) = request.git_branch {
        tracing::info!(
            "Switching to branch '{}' for project at: {}",
            git_branch,
            project.path
        );
        // Get worktree path for the specified branch
        let worktree_path =
            crate::git::get_working_directory_for_branch(Path::new(&project.path), git_branch)?;
        tracing::info!("Using worktree path: {}", worktree_path);
        Path::new(&worktree_path).to_path_buf()
    } else {
        tracing::info!(
            "No branch specified, using default project path: {}",
            project.path
        );
        Path::new(&project.path).to_path_buf()
    };

    let relative_path = request.path.as_deref().unwrap_or("");
    let full_path = project_path.join(relative_path);

    // Security check: ensure the path is within the project directory
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|e| AppError::InvalidRequest(format!("Invalid path: {e}")))?;
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    if !canonical_full_path.starts_with(&canonical_project_path) {
        return Err(AppError::InvalidRequest(
            "Access denied: path is outside project directory".to_string(),
        ));
    }

    if !canonical_full_path.exists() {
        return Err(AppError::InvalidRequest(
            "Directory does not exist".to_string(),
        ));
    }

    if !canonical_full_path.is_dir() {
        return Err(AppError::InvalidRequest(
            "Path is not a directory".to_string(),
        ));
    }

    // Read directory contents
    let entries = std::fs::read_dir(&canonical_full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read directory: {e}")))?;

    let mut files = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| AppError::Internal(format!("Failed to read entry: {e}")))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|e| AppError::Internal(format!("Failed to read metadata: {e}")))?;

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<invalid>")
            .to_string();

        // Calculate relative path from project root
        let relative_file_path = path
            .strip_prefix(&canonical_project_path)
            .map_err(|_| AppError::Internal("Failed to calculate relative path".to_string()))?;

        let is_directory = metadata.is_dir();
        let file_info = FileInfo {
            name,
            path: relative_file_path.to_string_lossy().to_string(),
            absolute: path.to_string_lossy().to_string(),
            file_type: if is_directory {
                FileType::Directory
            } else {
                FileType::File
            },
            ignored: false, // TODO: Implement .gitignore checking for API responses
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
                    t.duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string()
                })
            },
        };

        files.push(file_info);
    }

    // Sort files: directories first, then by name
    files.sort_by(|a, b| match (&a.file_type, &b.file_type) {
        (FileType::Directory, FileType::File) => std::cmp::Ordering::Less,
        (FileType::File, FileType::Directory) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    let total_files = files.len() as u32;
    let response = FileListResponse {
        files,
        current_path: relative_path.to_string(),
        total_files,
        truncated: false, // API doesn't implement truncation for now
        limit: 1000,      // Default limit
    };

    Ok(HttpResponse::Ok().json(response))
}

#[allow(dead_code)]
pub async fn create_file(
    data: web::Data<AppState>,
    request: web::Json<FileCreateRequest>,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(req.project_id)?;
    let project_path = Path::new(&project.path);
    let full_path = project_path.join(&req.path);

    // Security check: ensure the path is within the project directory
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    // Check if parent directory exists and resolve path
    if let Some(parent) = full_path.parent() {
        if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| AppError::InvalidRequest(format!("Invalid parent path: {e}")))?;

            if !canonical_parent.starts_with(&canonical_project_path) {
                return Err(AppError::InvalidRequest(
                    "Access denied: path is outside project directory".to_string(),
                ));
            }
        } else {
            return Err(AppError::InvalidRequest(
                "Parent directory does not exist".to_string(),
            ));
        }
    }

    // Check if file/directory already exists
    if full_path.exists() {
        return Err(AppError::InvalidRequest(
            "File or directory already exists".to_string(),
        ));
    }

    // Create file or directory
    if req.is_directory {
        std::fs::create_dir_all(&full_path)
            .map_err(|e| AppError::Internal(format!("Failed to create directory: {e}")))?;
    } else {
        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::Internal(format!("Failed to create parent directories: {e}"))
            })?;
        }

        let content = req.content.unwrap_or_default();
        std::fs::write(&full_path, content)
            .map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;
    }

    // Get file metadata for response
    let metadata = std::fs::metadata(&full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read metadata: {e}")))?;

    let file_name = full_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<invalid>")
        .to_string();

    let is_directory = metadata.is_dir();
    let file_info = FileInfo {
        name: file_name,
        path: req.path.clone(),
        absolute: full_path.to_string_lossy().to_string(),
        file_type: if is_directory {
            FileType::Directory
        } else {
            FileType::File
        },
        ignored: false, // New files are not ignored
        is_directory,
        size: if is_directory {
            None
        } else {
            metadata.len().into()
        },
        modified_at: if is_directory {
            None
        } else {
            metadata.modified().ok().map(|t| format!("{:?}", t))
        },
    };

    tracing::info!(
        "Created {} '{}' in project '{}'",
        if req.is_directory {
            "directory"
        } else {
            "file"
        },
        req.path,
        project.name
    );

    let response = FileResponse { file: file_info };
    Ok(HttpResponse::Created().json(response))
}

#[allow(dead_code)]
pub async fn get_file_content(
    data: web::Data<AppState>,
    path_param: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, AppError> {
    let file_path = urlencoding::decode(&path_param.into_inner())
        .map_err(|_| AppError::InvalidRequest("Invalid URL encoding in path".to_string()))?
        .to_string();
    let project_id_str = query
        .get("project_id")
        .ok_or_else(|| AppError::InvalidRequest("project_id is required".to_string()))?;
    let project_id = project_id_str
        .parse::<i64>()
        .map_err(|_| AppError::InvalidRequest("Invalid project_id".to_string()))?;

    let git_branch = query.get("git_branch").map(|s| s.as_str());

    tracing::info!(
        "get_file_content called for project_id: {}, path: {}, git_branch: {:?}",
        project_id,
        file_path,
        git_branch
    );

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(project_id)?;

    // Determine the base path - use worktree path if git_branch is specified
    let project_path = if let Some(branch) = git_branch {
        tracing::info!(
            "Switching to branch '{}' for file content in project: {}",
            branch,
            project.path
        );
        let worktree_path =
            crate::git::get_working_directory_for_branch(Path::new(&project.path), branch)?;
        tracing::info!("Using worktree path for file content: {}", worktree_path);
        Path::new(&worktree_path).to_path_buf()
    } else {
        tracing::info!(
            "No branch specified for file content, using default project path: {}",
            project.path
        );
        Path::new(&project.path).to_path_buf()
    };

    let full_path = project_path.join(&file_path);

    // Security check: ensure the path is within the project directory
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|e| AppError::InvalidRequest(format!("Invalid file path: {e}")))?;
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    if !canonical_full_path.starts_with(&canonical_project_path) {
        return Err(AppError::InvalidRequest(
            "Access denied: path is outside project directory".to_string(),
        ));
    }

    if !canonical_full_path.exists() {
        return Err(AppError::InvalidRequest("File does not exist".to_string()));
    }

    if !canonical_full_path.is_file() {
        return Err(AppError::InvalidRequest("Path is not a file".to_string()));
    }

    // Read file content
    let content = std::fs::read_to_string(&canonical_full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read file: {e}")))?;

    let metadata = std::fs::metadata(&canonical_full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read metadata: {e}")))?;

    let response = FileContentResponse {
        path: file_path,
        content,
        modified_at: metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
    };

    Ok(HttpResponse::Ok().json(response))
}

#[allow(dead_code)]
pub async fn update_file(
    data: web::Data<AppState>,
    path_param: web::Path<String>,
    request: web::Json<FileUpdateRequest>,
) -> Result<HttpResponse, AppError> {
    let file_path = urlencoding::decode(&path_param.into_inner())
        .map_err(|_| AppError::InvalidRequest("Invalid URL encoding in path".to_string()))?
        .to_string();
    let req = request.into_inner();

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(req.project_id)?;
    let project_path = Path::new(&project.path);
    let full_path = project_path.join(&file_path);

    // Security check: ensure the path is within the project directory
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|e| AppError::InvalidRequest(format!("Invalid file path: {e}")))?;
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    if !canonical_full_path.starts_with(&canonical_project_path) {
        return Err(AppError::InvalidRequest(
            "Access denied: path is outside project directory".to_string(),
        ));
    }

    if !canonical_full_path.exists() {
        return Err(AppError::InvalidRequest("File does not exist".to_string()));
    }

    if !canonical_full_path.is_file() {
        return Err(AppError::InvalidRequest("Path is not a file".to_string()));
    }

    // Write file content
    std::fs::write(&canonical_full_path, &req.content)
        .map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;

    let metadata = std::fs::metadata(&canonical_full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read metadata: {e}")))?;

    tracing::info!("Updated file '{}' in project '{}'", file_path, project.name);

    let response = FileContentResponse {
        path: file_path,
        content: req.content,
        modified_at: metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
    };

    Ok(HttpResponse::Ok().json(response))
}

#[allow(dead_code)]
pub async fn delete_file(
    data: web::Data<AppState>,
    path_param: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, AppError> {
    let file_path = urlencoding::decode(&path_param.into_inner())
        .map_err(|_| AppError::InvalidRequest("Invalid URL encoding in path".to_string()))?
        .to_string();
    let project_id_str = query
        .get("project_id")
        .ok_or_else(|| AppError::InvalidRequest("project_id is required".to_string()))?;
    let project_id = project_id_str
        .parse::<i64>()
        .map_err(|_| AppError::InvalidRequest("Invalid project_id".to_string()))?;

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(project_id)?;
    let project_path = Path::new(&project.path);
    let full_path = project_path.join(&file_path);

    // Security check: ensure the path is within the project directory
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|e| AppError::InvalidRequest(format!("Invalid file path: {e}")))?;
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    if !canonical_full_path.starts_with(&canonical_project_path) {
        return Err(AppError::InvalidRequest(
            "Access denied: path is outside project directory".to_string(),
        ));
    }

    if !canonical_full_path.exists() {
        return Err(AppError::InvalidRequest(
            "File or directory does not exist".to_string(),
        ));
    }

    // Delete file or directory
    if canonical_full_path.is_dir() {
        std::fs::remove_dir_all(&canonical_full_path)
            .map_err(|e| AppError::Internal(format!("Failed to remove directory: {e}")))?;
        tracing::info!(
            "Deleted directory '{}' from project '{}'",
            file_path,
            project.name
        );
    } else {
        std::fs::remove_file(&canonical_full_path)
            .map_err(|e| AppError::Internal(format!("Failed to remove file: {e}")))?;
        tracing::info!(
            "Deleted file '{}' from project '{}'",
            file_path,
            project.name
        );
    }

    Ok(HttpResponse::NoContent().finish())
}
