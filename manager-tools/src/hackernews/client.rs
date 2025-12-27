use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::types::hackernews::StoryType;

const BASE_URL: &str = "https://hacker-news.firebaseio.com/v0";
const TIMEOUT_SECS: u64 = 10;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnUser {
    pub id: String,
    pub created: i64,
    pub karma: i64,
    pub about: Option<String>,
    pub submitted: Option<Vec<i64>>,
}

#[derive(Clone)]
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

    pub async fn fetch_max_item_id(&self) -> Result<i64> {
        let url = format!("{}/maxitem.json", BASE_URL);
        let id = self.client.get(&url).send().await?.json::<i64>().await?;
        Ok(id)
    }

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

        all_ids.sort_unstable();
        all_ids.dedup();
        Ok(all_ids)
    }

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
