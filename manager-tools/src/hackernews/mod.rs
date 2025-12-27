use crate::types::{HackerNewsRequest, HackerNewsResponse, ToolResponse, StoryType, FetchMode, DownloadState};
use crate::tool_error::ToolError;
use anyhow::Result;

pub mod client;
pub mod fetcher;
pub mod storage;
pub mod schema;

pub use client::HnClient;
pub use fetcher::{ItemFetcher, FetchStats};

const DEFAULT_BATCH_SIZE: usize = 20;

pub async fn execute_hackernews_request(
    request: HackerNewsRequest,
) -> Result<ToolResponse, ToolError> {
    let batch_size = request.batch_size.unwrap_or(DEFAULT_BATCH_SIZE);

    match request.fetch_mode {
        FetchMode::StoryType { story_type } => {
            execute_story_type_mode(&request.db_path, story_type, batch_size).await
        }
        FetchMode::FetchAll => {
            execute_fetch_all_mode(&request.db_path, batch_size).await
        }
    }
}

async fn execute_story_type_mode(
    db_path: &str,
    story_type: StoryType,
    batch_size: usize,
) -> Result<ToolResponse, ToolError> {
    let client = HnClient::new()
        .map_err(|e| ToolError::ExecutionError(format!("Failed to create HN client: {}", e)))?;
    let fetcher = ItemFetcher::new(db_path, batch_size)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to create item fetcher: {}", e)))?;

    let item_ids = if matches!(story_type, StoryType::All) {
        client.fetch_all_story_ids().await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to fetch story IDs: {}", e)))?
    } else {
        client.fetch_story_ids(&story_type).await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to fetch story IDs: {}", e)))?
    };

    let stats = fetcher.fetch_items(item_ids.clone()).await
        .map_err(|e| ToolError::ExecutionError(format!("Failed to fetch items: {}", e)))?;

    let response = HackerNewsResponse {
        items_downloaded: stats.items_downloaded,
        users_downloaded: stats.users_downloaded,
        items_skipped: stats.items_skipped,
        items_processed: stats.items_processed(),
        has_more: false,
        state: DownloadState {
            mode: "story_type".to_string(),
            current_max_id: None,
            pending_story_types: vec![],
            in_progress_items: vec![],
            in_progress_users: vec![],
        },
        message: format!(
            "Downloaded {} items ({} new, {} skipped) and {} users for {:?} stories",
            stats.items_processed(),
            stats.items_downloaded,
            stats.items_skipped,
            stats.users_downloaded,
            story_type
        ),
    };

    Ok(ToolResponse::HackerNewsResponse(response))
}

async fn execute_fetch_all_mode(
    db_path: &str,
    batch_size: usize,
) -> Result<ToolResponse, ToolError> {
    let client = HnClient::new()
        .map_err(|e| ToolError::ExecutionError(format!("Failed to create HN client: {}", e)))?;
    let fetcher = ItemFetcher::new(db_path, batch_size)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to create item fetcher: {}", e)))?;

    let max_id = client.fetch_max_item_id().await
        .map_err(|e| ToolError::ExecutionError(format!("Failed to fetch max item ID: {}", e)))?;

    let start_id = max_id - batch_size as i64 + 1;
    let item_ids: Vec<i64> = (start_id.max(1)..=max_id).collect();

    let stats = fetcher.fetch_items(item_ids).await
        .map_err(|e| ToolError::ExecutionError(format!("Failed to fetch items: {}", e)))?;

    let response = HackerNewsResponse {
        items_downloaded: stats.items_downloaded,
        users_downloaded: stats.users_downloaded,
        items_skipped: stats.items_skipped,
        items_processed: stats.items_processed(),
        has_more: start_id > 1,
        state: DownloadState {
            mode: "fetch_all".to_string(),
            current_max_id: Some(start_id - 1),
            pending_story_types: vec![],
            in_progress_items: vec![],
            in_progress_users: vec![],
        },
        message: format!(
            "Downloaded batch from ID {} to {}: {} items ({} new, {} skipped), {} users. More: {}",
            start_id,
            max_id,
            stats.items_processed(),
            stats.items_downloaded,
            stats.items_skipped,
            stats.users_downloaded,
            start_id > 1
        ),
    };

    Ok(ToolResponse::HackerNewsResponse(response))
}
