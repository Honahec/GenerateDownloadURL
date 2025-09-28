use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::database::Database;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub tickets: Arc<RwLock<HashMap<Uuid, DownloadTicket>>>,
    pub database: Database,
}

impl AppState {
    pub fn new(config: AppConfig, database: Database) -> Self {
        Self {
            config: Arc::new(config),
            tickets: Arc::new(RwLock::new(HashMap::new())),
            database,
        }
    }
}

pub struct DownloadTicket {
    #[allow(dead_code)]
    pub id: Uuid,
    pub bucket_override: Option<String>,
    pub object_key: String,
    pub expires_at: DateTime<Utc>,
    pub max_downloads: Option<u32>,
    pub downloads_served: u32,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    pub download_filename: Option<String>,
    pub endpoint_override: Option<String>,
}
