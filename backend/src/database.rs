use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadLink {
    pub id: String,
    pub object_key: String,
    pub bucket: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub max_downloads: Option<i64>,
    pub downloads_served: i64,
    pub created_at: DateTime<Utc>,
    pub download_filename: Option<String>,
    pub endpoint: Option<String>,
    pub is_expired: bool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        use sqlx::sqlite::SqliteConnectOptions;
        use std::str::FromStr;

        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);
        let pool = SqlitePool::connect_with(options).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS download_links (
                id TEXT PRIMARY KEY NOT NULL,
                object_key TEXT NOT NULL,
                bucket TEXT,
                expires_at TEXT NOT NULL,
                max_downloads INTEGER,
                downloads_served INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                download_filename TEXT,
                endpoint TEXT
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Add endpoint column if it doesn't exist (for existing databases)
        let _ = sqlx::query("ALTER TABLE download_links ADD COLUMN endpoint TEXT")
            .execute(&pool)
            .await;

        // 创建索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_download_links_expires_at ON download_links(expires_at)")
            .execute(&pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_download_links_created_at ON download_links(created_at)")
            .execute(&pool)
            .await?;

        // 第二个迁移：添加 endpoint 列（如果不存在）
        sqlx::query("ALTER TABLE download_links ADD COLUMN endpoint TEXT")
            .execute(&pool)
            .await
            .ok(); // 忽略错误，因为列可能已经存在

        Ok(Self { pool })
    }

    pub async fn create_download_link(
        &self,
        id: Uuid,
        object_key: String,
        bucket: Option<String>,
        expires_at: DateTime<Utc>,
        max_downloads: Option<u32>,
        download_filename: Option<String>,
        endpoint: Option<String>,
    ) -> Result<()> {
        let expires_at_str = expires_at.to_rfc3339();
        let created_at_str = Utc::now().to_rfc3339();
        let max_downloads_i64 = max_downloads.map(|m| m as i64);

        sqlx::query(
            r#"
            INSERT INTO download_links (id, object_key, bucket, expires_at, max_downloads, downloads_served, created_at, download_filename, endpoint)
            VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?)
            "#
        )
        .bind(id.to_string())
        .bind(object_key)
        .bind(bucket)
        .bind(expires_at_str)
        .bind(max_downloads_i64)
        .bind(created_at_str)
        .bind(download_filename)
        .bind(endpoint)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_download_link(&self, id: &str) -> Result<Option<DownloadLink>> {
        let row = sqlx::query(
            "SELECT id, object_key, bucket, expires_at, max_downloads, downloads_served, created_at, download_filename, endpoint FROM download_links WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let expires_at_str: String = row.get("expires_at");
            let created_at_str: String = row.get("created_at");

            let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc);
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);

            let max_downloads: Option<i64> = row.get("max_downloads");
            let downloads_served: i64 = row.get("downloads_served");

            let now = Utc::now();
            let is_expired = expires_at < now
                || (max_downloads.is_some() && downloads_served >= max_downloads.unwrap());

            Ok(Some(DownloadLink {
                id: row.get("id"),
                object_key: row.get("object_key"),
                bucket: row.get("bucket"),
                expires_at,
                max_downloads,
                downloads_served,
                created_at,
                download_filename: row.get("download_filename"),
                endpoint: row.get("endpoint"),
                is_expired,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn increment_downloads(&self, id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE download_links SET downloads_served = downloads_served + 1 WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_download_links(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<DownloadLink>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let rows = sqlx::query(
            "SELECT id, object_key, bucket, expires_at, max_downloads, downloads_served, created_at, download_filename, endpoint 
             FROM download_links 
             ORDER BY created_at DESC 
             LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let now = Utc::now();
        let mut links = Vec::new();

        for row in rows {
            let expires_at_str: String = row.get("expires_at");
            let created_at_str: String = row.get("created_at");

            let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc);
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);

            let max_downloads: Option<i64> = row.get("max_downloads");
            let downloads_served: i64 = row.get("downloads_served");

            let is_expired = expires_at < now
                || (max_downloads.is_some() && downloads_served >= max_downloads.unwrap());

            links.push(DownloadLink {
                id: row.get("id"),
                object_key: row.get("object_key"),
                bucket: row.get("bucket"),
                expires_at,
                max_downloads,
                downloads_served,
                created_at,
                download_filename: row.get("download_filename"),
                endpoint: row.get("endpoint"),
                is_expired,
            });
        }

        Ok(links)
    }

    pub async fn delete_download_link(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM download_links WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_expired_links(&self) -> Result<u64> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "DELETE FROM download_links WHERE expires_at < ? OR (max_downloads IS NOT NULL AND downloads_served >= max_downloads)"
        )
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
