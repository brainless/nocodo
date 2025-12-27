# Add Hacker News Download Manager Tool to manager-tools

**Status**: ✅ Completed
**Priority**: Medium
**Created**: 2025-12-27
**Completed**: 2025-12-27

## Summary

Add a comprehensive Hacker News API download manager tool (`hackernews_request`) to manager-tools. This tool will download Items (stories, comments, polls), Users, and manage parallel fetching with SQLite storage and intelligent state tracking to avoid duplicate downloads.

## Problem Statement

Projects need to:
- Download and analyze Hacker News data programmatically
- Efficiently fetch stories, comments, and user profiles
- Store data in SQLite for offline analysis
- Avoid duplicate API requests
- Track download progress and state
- Handle parallel downloads efficiently

This tool will provide a reusable, efficient solution for downloading HN data with intelligent batching and state management.

## Goals

1. **Dual fetch modes**: Support both story-type fetching and fetch-all-from-max-ID modes
2. **Parallel downloads**: Batch Item fetching with configurable parallelism (default: 20)
3. **SQLite storage**: Store all Items and Users in a local SQLite database
4. **State tracking**: Track what's being downloaded to prevent duplicates
5. **Recursive fetching**: Auto-fetch Users and Comments for each Item
6. **Efficient batching**: Process downloads in manageable batches
7. **Schema migration**: Include DB schema management at tool level

## Architecture Overview

### Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Fetch modes** | Story type OR fetch all from max ID | Covers both curated lists and comprehensive downloads |
| **Batch size** | Default 20, configurable | Balance between parallelism and resource usage |
| **Storage** | SQLite database | Local, queryable, persistent storage |
| **State tracking** | In-memory + DB state | Prevents duplicate downloads during session |
| **API structure** | REST API (Firebase) | HN official API: `https://hacker-news.firebaseio.com/v0/` |
| **Parallel execution** | Tokio tasks with batching | Efficient async I/O for API requests |
| **Schema migration** | Embedded migrations | Tool manages its own schema |

### Tool Interface

```rust
/// Hacker News story type for fetching curated lists
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum StoryType {
    #[serde(rename = "top")]
    Top,      // /v0/topstories (500 items)

    #[serde(rename = "new")]
    New,      // /v0/newstories (500 items)

    #[serde(rename = "best")]
    Best,     // /v0/beststories (500 items)

    #[serde(rename = "ask")]
    Ask,      // /v0/askstories (200 items)

    #[serde(rename = "show")]
    Show,     // /v0/showstories (200 items)

    #[serde(rename = "job")]
    Job,      // /v0/jobstories (200 items)

    #[serde(rename = "all")]
    All,      // Fetch all curated story types listed above
}

/// Fetch mode for Hacker News download
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode")]
pub enum FetchMode {
    /// Fetch stories by type (top, new, best, ask, show, job, or all)
    #[serde(rename = "story_type")]
    StoryType { story_type: StoryType },

    /// Fetch all items starting from max item ID and walking backward
    #[serde(rename = "fetch_all")]
    FetchAll,
}

/// Request to download Hacker News data
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HackerNewsRequest {
    /// Database path to store downloaded data
    #[serde(default)]
    #[schemars(description = "Absolute path to SQLite database for storing HN data")]
    pub db_path: String,

    /// Fetch mode: story type or fetch all
    #[schemars(description = "Fetch mode: either specific story types or fetch all from max ID")]
    pub fetch_mode: FetchMode,

    /// Batch size for parallel downloads (default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Number of items to fetch in parallel per batch. Default: 20")]
    pub batch_size: Option<usize>,
}

/// Response from Hacker News download operation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HackerNewsResponse {
    /// Number of items downloaded in this batch
    pub items_downloaded: usize,

    /// Number of users downloaded in this batch
    pub users_downloaded: usize,

    /// Number of items skipped (already in DB)
    pub items_skipped: usize,

    /// Total items processed (downloaded + skipped)
    pub items_processed: usize,

    /// Whether more batches are pending
    pub has_more: bool,

    /// Current state of the download operation
    pub state: DownloadState,

    /// Human-readable status message
    pub message: String,
}

/// Download operation state
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DownloadState {
    /// Active fetch mode
    pub mode: String,

    /// For fetch_all mode: current max item ID being processed
    pub current_max_id: Option<i64>,

    /// For story_type mode: which story types are pending
    pub pending_story_types: Vec<String>,

    /// Item IDs currently being fetched
    pub in_progress_items: Vec<i64>,

    /// User IDs currently being fetched
    pub in_progress_users: Vec<String>,
}
```

## Database Schema

### Tables

```sql
-- Items table (stories, comments, polls, jobs)
CREATE TABLE IF NOT EXISTS items (
    id INTEGER PRIMARY KEY,
    type TEXT NOT NULL,  -- "story", "comment", "job", "poll", "pollopt"
    by TEXT,             -- Username of author (nullable for deleted items)
    time INTEGER NOT NULL,
    deleted INTEGER DEFAULT 0,
    dead INTEGER DEFAULT 0,
    parent INTEGER,      -- Parent item ID (for comments)
    poll INTEGER,        -- Poll ID (for pollopts)
    kids TEXT,           -- JSON array of child IDs
    url TEXT,
    score INTEGER,
    title TEXT,
    text TEXT,
    parts TEXT,          -- JSON array of pollopt IDs
    descendants INTEGER,
    fetched_at INTEGER NOT NULL,  -- Unix timestamp of when we fetched it
    FOREIGN KEY (by) REFERENCES users(id) ON DELETE SET NULL
);

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    created INTEGER NOT NULL,
    karma INTEGER NOT NULL,
    about TEXT,
    submitted TEXT,      -- JSON array of submitted item IDs
    fetched_at INTEGER NOT NULL
);

-- Download state tracking
CREATE TABLE IF NOT EXISTS download_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Single row table
    mode TEXT NOT NULL,                      -- "story_type" or "fetch_all"
    story_types TEXT,                        -- JSON array for story_type mode
    current_max_id INTEGER,                  -- For fetch_all mode
    batch_size INTEGER NOT NULL DEFAULT 20,
    updated_at INTEGER NOT NULL
);

-- In-progress tracking (prevents duplicate fetches in same session)
CREATE TABLE IF NOT EXISTS fetch_queue (
    item_id INTEGER PRIMARY KEY,
    queued_at INTEGER NOT NULL
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_items_by ON items(by);
CREATE INDEX IF NOT EXISTS idx_items_type ON items(type);
CREATE INDEX IF NOT EXISTS idx_items_parent ON items(parent);
CREATE INDEX IF NOT EXISTS idx_items_time ON items(time DESC);
```

## Implementation Plan

### Phase 1: Core Structure

#### 1.1 Create Module Structure

```
manager-tools/
  src/
    hackernews/
      mod.rs              # Public interface and executor
      client.rs           # HTTP client for HN API
      fetcher.rs          # Item and User fetching logic
      storage.rs          # SQLite storage operations
      state.rs            # Download state management
      schema.rs           # DB schema migrations
    types/
      hackernews.rs       # Request/Response types
```

#### 1.2 Create Type Definitions

**File**: `manager-tools/src/types/hackernews.rs`

Implement the types defined above in Tool Interface section.

#### 1.3 Update Type System

**File**: `manager-tools/src/types/core.rs`

Add variants:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ToolRequest {
    // ... existing variants
    #[serde(rename = "hackernews_request")]
    HackerNewsRequest(super::hackernews::HackerNewsRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResponse {
    // ... existing variants
    #[serde(rename = "hackernews_response")]
    HackerNewsResponse(super::hackernews::HackerNewsResponse),
}
```

**File**: `manager-tools/src/types/mod.rs`

Add:
```rust
pub mod hackernews;
pub use hackernews::{HackerNewsRequest, HackerNewsResponse, StoryType, FetchMode, DownloadState};
```

### Phase 2: API Client

#### 2.1 HN API Client

**File**: `manager-tools/src/hackernews/client.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const BASE_URL: &str = "https://hacker-news.firebaseio.com/v0";
const TIMEOUT_SECS: u64 = 10;

/// HN Item from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnItem {
    pub id: i64,
    #[serde(rename = "type")]
    pub item_type: String,
    pub by: Option<String>,
    pub time: i64,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub dead: bool,
    pub parent: Option<i64>,
    pub poll: Option<i64>,
    pub kids: Option<Vec<i64>>,
    pub url: Option<String>,
    pub score: Option<i64>,
    pub title: Option<String>,
    pub text: Option<String>,
    pub parts: Option<Vec<i64>>,
    pub descendants: Option<i64>,
}

/// HN User from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnUser {
    pub id: String,
    pub created: i64,
    pub karma: i64,
    pub about: Option<String>,
    pub submitted: Option<Vec<i64>>,
}

pub struct HnClient {
    client: reqwest::Client,
}

impl HnClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()?;
        Ok(Self { client })
    }

    /// Fetch max item ID
    pub async fn fetch_max_item_id(&self) -> Result<i64> {
        let url = format!("{}/maxitem.json", BASE_URL);
        let id = self.client.get(&url).send().await?.json::<i64>().await?;
        Ok(id)
    }

    /// Fetch story IDs by type
    pub async fn fetch_story_ids(&self, story_type: &StoryType) -> Result<Vec<i64>> {
        let endpoint = match story_type {
            StoryType::Top => "topstories",
            StoryType::New => "newstories",
            StoryType::Best => "beststories",
            StoryType::Ask => "askstories",
            StoryType::Show => "showstories",
            StoryType::Job => "jobstories",
            StoryType::All => return Err(anyhow::anyhow!("Use fetch_all_story_ids() for All")),
        };

        let url = format!("{}/{}.json", BASE_URL, endpoint);
        let ids = self.client.get(&url).send().await?.json::<Vec<i64>>().await?;
        Ok(ids)
    }

    /// Fetch all story type IDs (for StoryType::All)
    pub async fn fetch_all_story_ids(&self) -> Result<Vec<i64>> {
        let types = vec![
            StoryType::Top,
            StoryType::New,
            StoryType::Best,
            StoryType::Ask,
            StoryType::Show,
            StoryType::Job,
        ];

        let mut all_ids = Vec::new();
        for story_type in types {
            let ids = self.fetch_story_ids(&story_type).await?;
            all_ids.extend(ids);
        }

        // Deduplicate
        all_ids.sort_unstable();
        all_ids.dedup();
        Ok(all_ids)
    }

    /// Fetch single item by ID
    pub async fn fetch_item(&self, id: i64) -> Result<Option<HnItem>> {
        let url = format!("{}/item/{}.json", BASE_URL, id);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let item = response.json::<Option<HnItem>>().await?;
            Ok(item)
        } else {
            Ok(None)
        }
    }

    /// Fetch single user by ID
    pub async fn fetch_user(&self, id: &str) -> Result<Option<HnUser>> {
        let url = format!("{}/user/{}.json", BASE_URL, id);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let user = response.json::<Option<HnUser>>().await?;
            Ok(user)
        } else {
            Ok(None)
        }
    }
}
```

### Phase 3: Database Storage

#### 3.1 Schema Migration

**File**: `manager-tools/src/hackernews/schema.rs`

```rust
use rusqlite::Connection;
use anyhow::Result;

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
        -- Items table
        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL,
            by TEXT,
            time INTEGER NOT NULL,
            deleted INTEGER DEFAULT 0,
            dead INTEGER DEFAULT 0,
            parent INTEGER,
            poll INTEGER,
            kids TEXT,
            url TEXT,
            score INTEGER,
            title TEXT,
            text TEXT,
            parts TEXT,
            descendants INTEGER,
            fetched_at INTEGER NOT NULL
        );

        -- Users table
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            created INTEGER NOT NULL,
            karma INTEGER NOT NULL,
            about TEXT,
            submitted TEXT,
            fetched_at INTEGER NOT NULL
        );

        -- Download state
        CREATE TABLE IF NOT EXISTS download_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            mode TEXT NOT NULL,
            story_types TEXT,
            current_max_id INTEGER,
            batch_size INTEGER NOT NULL DEFAULT 20,
            updated_at INTEGER NOT NULL
        );

        -- Fetch queue
        CREATE TABLE IF NOT EXISTS fetch_queue (
            item_id INTEGER PRIMARY KEY,
            queued_at INTEGER NOT NULL
        );

        -- Indexes
        CREATE INDEX IF NOT EXISTS idx_items_by ON items(by);
        CREATE INDEX IF NOT EXISTS idx_items_type ON items(type);
        CREATE INDEX IF NOT EXISTS idx_items_parent ON items(parent);
        CREATE INDEX IF NOT EXISTS idx_items_time ON items(time DESC);
    "#)?;

    Ok(())
}
```

#### 3.2 Storage Operations

**File**: `manager-tools/src/hackernews/storage.rs`

```rust
use rusqlite::{Connection, params};
use anyhow::Result;
use super::client::{HnItem, HnUser};

pub struct HnStorage {
    conn: Connection,
}

impl HnStorage {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        super::schema::initialize_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Check if item exists
    pub fn item_exists(&self, id: i64) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM items WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Check if user exists
    pub fn user_exists(&self, id: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM users WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Save item to database
    pub fn save_item(&self, item: &HnItem) -> Result<()> {
        let fetched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO items (
                id, type, by, time, deleted, dead, parent, poll,
                kids, url, score, title, text, parts, descendants, fetched_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            "#,
            params![
                item.id,
                item.item_type,
                item.by,
                item.time,
                item.deleted as i64,
                item.dead as i64,
                item.parent,
                item.poll,
                item.kids.as_ref().map(|k| serde_json::to_string(k).unwrap()),
                item.url,
                item.score,
                item.title,
                item.text,
                item.parts.as_ref().map(|p| serde_json::to_string(p).unwrap()),
                item.descendants,
                fetched_at,
            ],
        )?;

        Ok(())
    }

    /// Save user to database
    pub fn save_user(&self, user: &HnUser) -> Result<()> {
        let fetched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO users (
                id, created, karma, about, submitted, fetched_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                user.id,
                user.created,
                user.karma,
                user.about,
                user.submitted.as_ref().map(|s| serde_json::to_string(s).unwrap()),
                fetched_at,
            ],
        )?;

        Ok(())
    }

    /// Mark items as queued
    pub fn queue_items(&self, item_ids: &[i64]) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        for id in item_ids {
            self.conn.execute(
                "INSERT OR IGNORE INTO fetch_queue (item_id, queued_at) VALUES (?1, ?2)",
                params![id, now],
            )?;
        }
        Ok(())
    }

    /// Remove items from queue
    pub fn dequeue_items(&self, item_ids: &[i64]) -> Result<()> {
        for id in item_ids {
            self.conn.execute(
                "DELETE FROM fetch_queue WHERE item_id = ?1",
                params![id],
            )?;
        }
        Ok(())
    }

    /// Get queued item IDs
    pub fn get_queued_items(&self) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare("SELECT item_id FROM fetch_queue")?;
        let ids = stmt.query_map([], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }
}
```

### Phase 4: Fetcher Logic

#### 4.1 Item Fetcher

**File**: `manager-tools/src/hackernews/fetcher.rs`

```rust
use anyhow::Result;
use super::client::{HnClient, HnItem, HnUser};
use super::storage::HnStorage;
use tokio::task::JoinSet;

pub struct ItemFetcher {
    client: HnClient,
    storage: HnStorage,
    batch_size: usize,
}

impl ItemFetcher {
    pub fn new(db_path: &str, batch_size: usize) -> Result<Self> {
        Ok(Self {
            client: HnClient::new()?,
            storage: HnStorage::new(db_path)?,
            batch_size,
        })
    }

    /// Fetch items in parallel batches
    pub async fn fetch_items(&self, item_ids: Vec<i64>) -> Result<FetchStats> {
        let mut stats = FetchStats::default();

        // Filter out already-fetched items
        let mut to_fetch = Vec::new();
        for id in item_ids {
            if self.storage.item_exists(id)? {
                stats.items_skipped += 1;
            } else {
                to_fetch.push(id);
            }
        }

        // Process in batches
        for batch in to_fetch.chunks(self.batch_size) {
            let batch_stats = self.fetch_batch(batch).await?;
            stats.merge(batch_stats);
        }

        Ok(stats)
    }

    /// Fetch a single batch
    async fn fetch_batch(&self, item_ids: &[i64]) -> Result<FetchStats> {
        let mut stats = FetchStats::default();
        let mut tasks = JoinSet::new();

        // Queue items
        self.storage.queue_items(item_ids)?;

        // Spawn fetch tasks
        for &id in item_ids {
            let client = self.client.clone();
            tasks.spawn(async move {
                client.fetch_item(id).await
            });
        }

        // Collect results
        let mut items_to_save = Vec::new();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(Some(item))) => {
                    items_to_save.push(item);
                }
                Ok(Ok(None)) => {
                    // Item doesn't exist or deleted
                    stats.items_skipped += 1;
                }
                Ok(Err(e)) => {
                    eprintln!("Error fetching item: {}", e);
                }
                Err(e) => {
                    eprintln!("Task join error: {}", e);
                }
            }
        }

        // Save items and extract user IDs & comment IDs
        let mut user_ids = Vec::new();
        let mut comment_ids = Vec::new();

        for item in &items_to_save {
            self.storage.save_item(item)?;
            stats.items_downloaded += 1;

            if let Some(ref by) = item.by {
                user_ids.push(by.clone());
            }

            if let Some(ref kids) = item.kids {
                comment_ids.extend(kids.iter().copied());
            }
        }

        // Dequeue processed items
        self.storage.dequeue_items(item_ids)?;

        // Fetch users
        let user_stats = self.fetch_users(user_ids).await?;
        stats.merge(user_stats);

        // Recursively fetch comments (if any)
        if !comment_ids.is_empty() {
            let comment_stats = self.fetch_items(comment_ids).await?;
            stats.merge(comment_stats);
        }

        Ok(stats)
    }

    /// Fetch users in parallel
    async fn fetch_users(&self, user_ids: Vec<String>) -> Result<FetchStats> {
        let mut stats = FetchStats::default();
        let mut tasks = JoinSet::new();

        // Filter already-fetched users
        let mut to_fetch = Vec::new();
        for id in user_ids {
            if !self.storage.user_exists(&id)? {
                to_fetch.push(id);
            }
        }

        // Spawn user fetch tasks
        for id in to_fetch {
            let client = self.client.clone();
            tasks.spawn(async move {
                client.fetch_user(&id).await
            });
        }

        // Collect and save users
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(Some(user))) => {
                    self.storage.save_user(&user)?;
                    stats.users_downloaded += 1;
                }
                Ok(Ok(None)) => {
                    // User doesn't exist
                }
                Ok(Err(e)) => {
                    eprintln!("Error fetching user: {}", e);
                }
                Err(e) => {
                    eprintln!("Task join error: {}", e);
                }
            }
        }

        Ok(stats)
    }
}

#[derive(Default, Debug)]
pub struct FetchStats {
    pub items_downloaded: usize,
    pub users_downloaded: usize,
    pub items_skipped: usize,
}

impl FetchStats {
    pub fn merge(&mut self, other: FetchStats) {
        self.items_downloaded += other.items_downloaded;
        self.users_downloaded += other.users_downloaded;
        self.items_skipped += other.items_skipped;
    }

    pub fn items_processed(&self) -> usize {
        self.items_downloaded + self.items_skipped
    }
}
```

### Phase 5: Main Executor

#### 5.1 Tool Executor

**File**: `manager-tools/src/hackernews/mod.rs`

```rust
use crate::types::{HackerNewsRequest, HackerNewsResponse, ToolResponse, StoryType, FetchMode, DownloadState};
use crate::tool_error::ToolError;
use anyhow::Result;

mod client;
mod fetcher;
mod storage;
mod schema;

use client::HnClient;
use fetcher::ItemFetcher;

const DEFAULT_BATCH_SIZE: usize = 20;

pub async fn execute_hackernews_request(
    request: HackerNewsRequest,
) -> Result<ToolResponse> {
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
) -> Result<ToolResponse> {
    let client = HnClient::new()?;
    let fetcher = ItemFetcher::new(db_path, batch_size)?;

    // Fetch story IDs
    let item_ids = if matches!(story_type, StoryType::All) {
        client.fetch_all_story_ids().await?
    } else {
        client.fetch_story_ids(&story_type).await?
    };

    // Fetch items
    let stats = fetcher.fetch_items(item_ids.clone()).await?;

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
) -> Result<ToolResponse> {
    let client = HnClient::new()?;
    let fetcher = ItemFetcher::new(db_path, batch_size)?;

    // Get max item ID
    let max_id = client.fetch_max_item_id().await?;

    // Create batch from max_id backward
    let start_id = max_id - batch_size as i64 + 1;
    let item_ids: Vec<i64> = (start_id.max(1)..=max_id).collect();

    // Fetch items
    let stats = fetcher.fetch_items(item_ids).await?;

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
```

#### 5.2 Integrate with ToolExecutor

**File**: `manager-tools/src/tool_executor.rs`

Add import:
```rust
use crate::hackernews;
```

Add match arm in `execute()`:
```rust
pub async fn execute(&self, request: ToolRequest) -> Result<ToolResponse> {
    match request {
        // ... existing arms
        ToolRequest::HackerNewsRequest(req) => {
            hackernews::execute_hackernews_request(req).await
        }
    }
}
```

Add match arm in `execute_from_json()`:
```rust
let response_value = match tool_response {
    // ... existing arms
    ToolResponse::HackerNewsResponse(response) => serde_json::to_value(response)?,
};
```

### Phase 6: Dependencies

**File**: `manager-tools/Cargo.toml`

Add:
```toml
[dependencies]
# ... existing
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
rusqlite = "0.32"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
```

### Phase 7: Update Module Exports

**File**: `manager-tools/src/lib.rs`

Add:
```rust
pub mod hackernews;
```

## Testing Strategy

### Unit Tests

1. **client.rs**: Test API client methods
   - `test_fetch_max_item_id()`
   - `test_fetch_story_ids()`
   - `test_fetch_item()`
   - `test_fetch_user()`

2. **storage.rs**: Test DB operations
   - `test_save_and_retrieve_item()`
   - `test_save_and_retrieve_user()`
   - `test_item_exists()`
   - `test_queue_dequeue()`

3. **fetcher.rs**: Test fetch logic
   - `test_fetch_batch()`
   - `test_fetch_users()`
   - `test_skip_existing_items()`

## Files Changed

### New Files
- `manager-tools/src/hackernews/mod.rs`
- `manager-tools/src/hackernews/client.rs`
- `manager-tools/src/hackernews/fetcher.rs`
- `manager-tools/src/hackernews/storage.rs`
- `manager-tools/src/hackernews/schema.rs`
- `manager-tools/src/types/hackernews.rs`
- `manager-tools/tasks/add-hackernews-download-manager.md`

### Modified Files
- `manager-tools/Cargo.toml`
- `manager-tools/src/lib.rs`
- `manager-tools/src/types/mod.rs`
- `manager-tools/src/types/core.rs`
- `manager-tools/src/tool_executor.rs`

## Success Criteria

- [ ] HackerNewsRequest tool integrated into manager-tools
- [ ] Story type mode works for all types (Top, New, Best, Ask, Show, Job, All)
- [ ] Fetch all mode works and walks backward from max ID
- [ ] Parallel fetching with configurable batch size
- [ ] Items, Users, and Comments stored in SQLite
- [ ] Duplicate prevention works
- [ ] Recursive comment fetching works
- [ ] Schema migrations work
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Documentation complete

## Usage Example

```rust
use manager_tools::{ToolExecutor, ToolRequest, HackerNewsRequest, FetchMode, StoryType};

// Fetch top stories
let request = ToolRequest::HackerNewsRequest(HackerNewsRequest {
    db_path: "/tmp/hackernews.db".to_string(),
    fetch_mode: FetchMode::StoryType {
        story_type: StoryType::Top,
    },
    batch_size: Some(20),
});

let response = executor.execute(request).await?;

// Fetch all from max ID
let request = ToolRequest::HackerNewsRequest(HackerNewsRequest {
    db_path: "/tmp/hackernews.db".to_string(),
    fetch_mode: FetchMode::FetchAll,
    batch_size: Some(50),
});

let response = executor.execute(request).await?;
```

## References

- **HN API Docs**: ~/Projects/API/README.md
- **HN API Base URL**: https://hacker-news.firebaseio.com/v0/
- **Reqwest**: https://docs.rs/reqwest/
- **Tokio**: https://docs.rs/tokio/
- **Rusqlite**: https://docs.rs/rusqlite/

## Notes

- The tool runs continuously in "fetch all" mode until the app exits
- Batch size is configurable but defaults to 20 for good balance
- All fetching is recursive: Item → User + Comments → Users of Comments → ...
- State tracking prevents duplicate downloads during a session
- DB schema is created automatically on first run
- The tool is designed to be resumed - it won't re-fetch existing items
