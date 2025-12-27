use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const DEFAULT_DB_PATH: &str = "/tmp/hackernews.db";

fn default_db_path() -> String {
    DEFAULT_DB_PATH.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum StoryType {
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "new")]
    New,
    #[serde(rename = "best")]
    Best,
    #[serde(rename = "ask")]
    Ask,
    #[serde(rename = "show")]
    Show,
    #[serde(rename = "job")]
    Job,
    #[serde(rename = "all")]
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode")]
pub enum FetchMode {
    #[serde(rename = "story_type")]
    StoryType { story_type: StoryType },
    #[serde(rename = "fetch_all")]
    FetchAll,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HackerNewsRequest {
    #[serde(default = "default_db_path")]
    #[schemars(description = "Absolute path to SQLite database for storing HN data")]
    pub db_path: String,
    #[schemars(description = "Fetch mode: either specific story types or fetch all from max ID")]
    pub fetch_mode: FetchMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Number of items to fetch in parallel per batch. Default: 20")]
    pub batch_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Maximum recursion depth for comment fetching. Default: 5")]
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HackerNewsResponse {
    pub items_downloaded: usize,
    pub users_downloaded: usize,
    pub items_skipped: usize,
    pub items_failed: usize,
    pub users_failed: usize,
    pub items_processed: usize,
    pub has_more: bool,
    pub state: DownloadState,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DownloadState {
    pub mode: String,
    pub current_max_id: Option<i64>,
    pub pending_story_types: Vec<String>,
    pub in_progress_items: Vec<i64>,
    pub in_progress_users: Vec<String>,
}
