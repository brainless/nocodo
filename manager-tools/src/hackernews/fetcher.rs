use anyhow::Result;
use super::client::HnClient;
use super::storage::HnStorage;
use tokio::task::JoinSet;
use std::sync::Arc;

pub struct ItemFetcher {
    client: HnClient,
    storage: Arc<HnStorage>,
    batch_size: usize,
    max_depth: usize,
}

impl ItemFetcher {
    pub fn new(db_path: &str, batch_size: usize, max_depth: usize) -> Result<Self> {
        Ok(Self {
            client: HnClient::new()?,
            storage: Arc::new(HnStorage::new(db_path)?),
            batch_size,
            max_depth,
        })
    }

    pub async fn fetch_items(&self, item_ids: Vec<i64>) -> Result<FetchStats> {
        self.fetch_items_internal(item_ids, 0).await
    }

    async fn fetch_items_internal(&self, item_ids: Vec<i64>, depth: usize) -> Result<FetchStats> {
        let mut stats = FetchStats::default();

        let mut to_fetch = Vec::new();
        for id in item_ids {
            let storage = Arc::clone(&self.storage);
            let exists = tokio::task::spawn_blocking(move || storage.item_exists(id)).await??;
            if exists {
                tracing::debug!("Skipping item {} (already exists, depth={})", id, depth);
                stats.items_skipped += 1;
            } else {
                to_fetch.push(id);
            }
        }

        if !to_fetch.is_empty() {
            tracing::info!("Fetching {} items at depth {}", to_fetch.len(), depth);
        }

        for batch in to_fetch.chunks(self.batch_size) {
            let batch_stats = self.fetch_batch_internal(batch, depth).await?;
            stats.merge(batch_stats);
        }

        Ok(stats)
    }

    async fn fetch_batch_internal(&self, item_ids: &[i64], depth: usize) -> Result<FetchStats> {
        let mut stats = FetchStats::default();
        let mut tasks = JoinSet::new();

        {
            let storage = Arc::clone(&self.storage);
            let ids = item_ids.to_vec();
            tokio::task::spawn_blocking(move || storage.queue_items(&ids)).await??;
        }

        for &id in item_ids {
            let client = self.client.clone();
            tasks.spawn(async move {
                client.fetch_item(id).await
            });
        }

        let mut items_to_save = Vec::new();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(Some(item))) => {
                    items_to_save.push(item);
                }
                Ok(Ok(None)) => {
                    tracing::debug!("Item returned None (deleted or doesn't exist)");
                    stats.items_skipped += 1;
                }
                Ok(Err(e)) => {
                    tracing::warn!("Error fetching item: {}", e);
                    stats.items_failed += 1;
                }
                Err(e) => {
                    tracing::warn!("Task join error: {}", e);
                    stats.items_failed += 1;
                }
            }
        }

        let mut user_ids = Vec::new();
        let mut comment_ids = Vec::new();

        for item in &items_to_save {
            {
                let storage = Arc::clone(&self.storage);
                let item_clone = item.clone();
                tokio::task::spawn_blocking(move || storage.save_item(&item_clone)).await??;
            }
            stats.items_downloaded += 1;

            tracing::info!(
                "Downloaded {} {} by {}",
                item.item_type,
                item.id,
                item.by.as_deref().unwrap_or("unknown")
            );

            if let Some(ref by) = item.by {
                user_ids.push(by.clone());
            }

            if let Some(ref kids) = item.kids {
                comment_ids.extend(kids.iter().copied());
            }
        }

        {
            let storage = Arc::clone(&self.storage);
            let ids = item_ids.to_vec();
            tokio::task::spawn_blocking(move || storage.dequeue_items(&ids)).await??;
        }

        let user_stats = self.fetch_users(user_ids).await?;
        stats.merge(user_stats);

        if !comment_ids.is_empty() && depth < self.max_depth {
            let comment_stats = Box::pin(self.fetch_items_internal(comment_ids, depth + 1)).await?;
            stats.merge(comment_stats);
        }

        Ok(stats)
    }

    async fn fetch_users(&self, user_ids: Vec<String>) -> Result<FetchStats> {
        let mut stats = FetchStats::default();
        let mut tasks = JoinSet::new();

        let mut to_fetch = Vec::new();
        for id in user_ids {
            let storage = Arc::clone(&self.storage);
            let id_clone = id.clone();
            let exists = tokio::task::spawn_blocking(move || storage.user_exists(&id_clone)).await??;
            if !exists {
                to_fetch.push(id);
            } else {
                tracing::debug!("Skipping user {} (already exists)", id);
            }
        }

        if !to_fetch.is_empty() {
            tracing::info!("Fetching {} users", to_fetch.len());
        }

        for id in to_fetch {
            let client = self.client.clone();
            let id_clone = id.clone();
            tasks.spawn(async move {
                let result = client.fetch_user(&id_clone).await;
                (id_clone, result)
            });
        }

        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((user_id, Ok(Some(user)))) => {
                    {
                        let storage = Arc::clone(&self.storage);
                        let user_clone = user.clone();
                        tokio::task::spawn_blocking(move || storage.save_user(&user_clone)).await??;
                    }
                    stats.users_downloaded += 1;
                    tracing::info!("Downloaded user {} (karma: {})", user_id, user.karma);
                }
                Ok((user_id, Ok(None))) => {
                    tracing::debug!("User {} returned None", user_id);
                }
                Ok((user_id, Err(e))) => {
                    tracing::warn!("Error fetching user {}: {}", user_id, e);
                    stats.users_failed += 1;
                }
                Err(e) => {
                    tracing::warn!("Task join error: {}", e);
                    stats.users_failed += 1;
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
    pub items_failed: usize,
    pub users_failed: usize,
}

impl FetchStats {
    pub fn merge(&mut self, other: FetchStats) {
        self.items_downloaded += other.items_downloaded;
        self.users_downloaded += other.users_downloaded;
        self.items_skipped += other.items_skipped;
        self.items_failed += other.items_failed;
        self.users_failed += other.users_failed;
    }

    pub fn items_processed(&self) -> usize {
        self.items_downloaded + self.items_skipped + self.items_failed
    }
}
