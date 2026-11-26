use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub http_host: String,
    pub http_port: u16,
    pub db_path: String,
    pub recipes_dir: String,
    pub scrape_interval_secs: u64,
    pub max_concurrent_tasks: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            http_host: "127.0.0.1".to_string(),
            http_port: 8089,
            db_path: "scraper.sqlite".to_string(),
            recipes_dir: "recipes".to_string(),
            scrape_interval_secs: 300, // 5 menit per siklus
            max_concurrent_tasks: 4,
        }
    }
}
