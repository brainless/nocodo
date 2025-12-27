use anyhow::Result;
use super::client::HnClient;
use super::storage::HnStorage;
use tokio::task::JoinSet;

pub struct ItemFetcher {
    client: HnClient,
    storage: HnStorage,
    batch_size: usize,
    max_depth: usize,
}

impl ItemFetcher {
    pub fn new(db_path: &str, batch_size: usize, max_depth: usize) -> Result<Self> {
        Ok(Self {
            client: HnClient::new()?,
            storage: HnStorage::new(db_path)?,
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
            if self.storage.item_exists(id)? {
                stats.items_skipped += 1;
            } else {
                to_fetch.push(id);
            }
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

        self.storage.queue_items(item_ids)?;

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
                    stats.items_skipped += 1;
                }
                Ok(Err(e)) => {
                    eprintln!("Error fetching item: {}", e);
                    stats.items_failed += 1;
                }
                Err(e) => {
                    eprintln!("Task join error: {}", e);
                    stats.items_failed += 1;
                }
            }
        }

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

        self.storage.dequeue_items(item_ids)?;

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
            if !self.storage.user_exists(&id)? {
                to_fetch.push(id);
            }
        }

        for id in to_fetch {
            let client = self.client.clone();
            tasks.spawn(async move {
                client.fetch_user(&id).await
            });
        }

        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(Some(user))) => {
                    self.storage.save_user(&user)?;
                    stats.users_downloaded += 1;
                }
                Ok(Ok(None)) => {}
                Ok(Err(e)) => {
                    eprintln!("Error fetching user: {}", e);
                    stats.users_failed += 1;
                }
                Err(e) => {
                    eprintln!("Task join error: {}", e);
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
