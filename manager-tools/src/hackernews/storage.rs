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

    pub fn item_exists(&self, id: i64) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM items WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn user_exists(&self, id: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM users WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

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

    pub fn dequeue_items(&self, item_ids: &[i64]) -> Result<()> {
        for id in item_ids {
            self.conn.execute(
                "DELETE FROM fetch_queue WHERE item_id = ?1",
                params![id],
            )?;
        }
        Ok(())
    }

    pub fn get_queued_items(&self) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare("SELECT item_id FROM fetch_queue")?;
        let ids = stmt.query_map([], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }
}
