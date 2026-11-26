use crate::schema::INIT_SQL;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use eay_scraper_core::error::{Result, ScraperError};
use eay_scraper_core::model::Article;
use eay_scraper_core::pipeline::StorageSink;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// Implementasi SQLite StorageSink thread-safe dengan deduplikasi URL Hash O(1)
#[derive(Clone)]
pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    /// Buat atau buka database SQLite pada file path tertentu
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| ScraperError::Storage(format!("Failed to open SQLite database: {}", e)))?;

        // Jalankan migrasi tabel
        conn.execute_batch(INIT_SQL)
            .map_err(|e| ScraperError::Storage(format!("Failed to run schema init: {}", e)))?;

        // Optimasi SQLite untuk scraping berkecepatan tinggi
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -64000;
            "#,
        )
        .map_err(|e| ScraperError::Storage(format!("Failed to set PRAGMAs: {}", e)))?;

        info!("SQLite storage initialized successfully");
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Buat database in-memory untuk pengujian
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ScraperError::Storage(format!("Failed to open in-memory SQLite: {}", e)))?;

        conn.execute_batch(INIT_SQL)
            .map_err(|e| ScraperError::Storage(format!("Failed to run schema init: {}", e)))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl StorageSink for SqliteStorage {
    async fn contains_hash(&self, url_hash: &str) -> Result<bool> {
        let hash = url_hash.to_string();
        let conn_arc = Arc::clone(&self.conn);

        // Offload sinkronus SQLite call ke blocking task untuk menjaga async runtime
        tokio::task::spawn_blocking(move || {
            let conn = conn_arc.lock().map_err(|_| {
                ScraperError::Storage("Mutex poison error on SQLite connection".into())
            })?;

            let mut stmt = conn
                .prepare("SELECT 1 FROM articles WHERE url_hash = ?1 LIMIT 1")
                .map_err(|e| ScraperError::Storage(e.to_string()))?;

            let exists = stmt
                .exists(params![hash])
                .map_err(|e| ScraperError::Storage(e.to_string()))?;

            Ok(exists)
        })
        .await
        .map_err(|e| ScraperError::Internal(format!("Join error: {}", e)))?
    }

    async fn save_article(&self, article: &Article) -> Result<()> {
        let article_clone = article.clone();
        let conn_arc = Arc::clone(&self.conn);

        tokio::task::spawn_blocking(move || {
            let conn = conn_arc.lock().map_err(|_| {
                ScraperError::Storage("Mutex poison error on SQLite connection".into())
            })?;

            let tags_json = serde_json::to_string(&article_clone.tags)
                .unwrap_or_else(|_| "[]".to_string());
            let metadata_json = serde_json::to_string(&article_clone.metadata)
                .unwrap_or_else(|_| "{}".to_string());
            let published_str = article_clone
                .published_at
                .map(|dt| dt.to_rfc3339());
            let scraped_str = article_clone.scraped_at.to_rfc3339();

            conn.execute(
                r#"
                INSERT OR IGNORE INTO articles (
                    id, provider_id, original_url, canonical_url, url_hash,
                    title, byline, summary, content_text, content_html,
                    published_at, scraped_at, top_image, tags_json, language, metadata_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                "#,
                params![
                    article_clone.id,
                    article_clone.provider_id,
                    article_clone.original_url,
                    article_clone.canonical_url,
                    article_clone.url_hash,
                    article_clone.title,
                    article_clone.byline,
                    article_clone.summary,
                    article_clone.content_text,
                    article_clone.content_html,
                    published_str,
                    scraped_str,
                    article_clone.top_image,
                    tags_json,
                    article_clone.language,
                    metadata_json,
                ],
            )
            .map_err(|e| ScraperError::Storage(format!("Failed to insert article: {}", e)))?;

            debug!("Saved article: [{}] {}", article_clone.provider_id, article_clone.title);
            Ok(())
        })
        .await
        .map_err(|e| ScraperError::Internal(format!("Join error: {}", e)))?
    }

    async fn get_articles(
        &self,
        provider_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Article>> {
        let provider_id = provider_id.map(|s| s.to_string());
        let conn_arc = Arc::clone(&self.conn);

        tokio::task::spawn_blocking(move || {
            let conn = conn_arc.lock().map_err(|_| {
                ScraperError::Storage("Mutex poison error on SQLite connection".into())
            })?;

            let mut articles = Vec::new();

            if let Some(pid) = provider_id {
                let mut stmt = conn
                    .prepare(
                        r#"
                        SELECT id, provider_id, original_url, canonical_url, url_hash,
                               title, byline, summary, content_text, content_html,
                               published_at, scraped_at, top_image, tags_json, language, metadata_json
                        FROM articles
                        WHERE provider_id = ?1
                        ORDER BY scraped_at DESC
                        LIMIT ?2 OFFSET ?3
                        "#,
                    )
                    .map_err(|e| ScraperError::Storage(e.to_string()))?;

                let rows = stmt
                    .query_map(params![pid, limit as i64, offset as i64], row_to_article)
                    .map_err(|e| ScraperError::Storage(e.to_string()))?;

                for r in rows {
                    if let Ok(art) = r {
                        articles.push(art);
                    }
                }
            } else {
                let mut stmt = conn
                    .prepare(
                        r#"
                        SELECT id, provider_id, original_url, canonical_url, url_hash,
                               title, byline, summary, content_text, content_html,
                               published_at, scraped_at, top_image, tags_json, language, metadata_json
                        FROM articles
                        ORDER BY scraped_at DESC
                        LIMIT ?1 OFFSET ?2
                        "#,
                    )
                    .map_err(|e| ScraperError::Storage(e.to_string()))?;

                let rows = stmt
                    .query_map(params![limit as i64, offset as i64], row_to_article)
                    .map_err(|e| ScraperError::Storage(e.to_string()))?;

                for r in rows {
                    if let Ok(art) = r {
                        articles.push(art);
                    }
                }
            }

            Ok(articles)
        })
        .await
        .map_err(|e| ScraperError::Internal(format!("Join error: {}", e)))?
    }

    async fn get_article_by_id(&self, id: &str) -> Result<Option<Article>> {
        let id_str = id.to_string();
        let conn_arc = Arc::clone(&self.conn);

        tokio::task::spawn_blocking(move || {
            let conn = conn_arc.lock().map_err(|_| {
                ScraperError::Storage("Mutex poison error on SQLite connection".into())
            })?;

            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, provider_id, original_url, canonical_url, url_hash,
                           title, byline, summary, content_text, content_html,
                           published_at, scraped_at, top_image, tags_json, language, metadata_json
                    FROM articles
                    WHERE id = ?1
                    LIMIT 1
                    "#,
                )
                .map_err(|e| ScraperError::Storage(e.to_string()))?;

            let mut rows = stmt
                .query_map(params![id_str], row_to_article)
                .map_err(|e| ScraperError::Storage(e.to_string()))?;

            if let Some(Ok(art)) = rows.next() {
                Ok(Some(art))
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| ScraperError::Internal(format!("Join error: {}", e)))?
    }

    async fn count_articles(&self, provider_id: Option<&str>) -> Result<usize> {
        let provider_id = provider_id.map(|s| s.to_string());
        let conn_arc = Arc::clone(&self.conn);

        tokio::task::spawn_blocking(move || {
            let conn = conn_arc.lock().map_err(|_| {
                ScraperError::Storage("Mutex poison error on SQLite connection".into())
            })?;

            let count: i64 = if let Some(pid) = provider_id {
                conn.query_row(
                    "SELECT COUNT(*) FROM articles WHERE provider_id = ?1",
                    params![pid],
                    |r| r.get(0),
                )
                .unwrap_or(0)
            } else {
                conn.query_row("SELECT COUNT(*) FROM articles", [], |r| r.get(0))
                    .unwrap_or(0)
            };

            Ok(count as usize)
        })
        .await
        .map_err(|e| ScraperError::Internal(format!("Join error: {}", e)))?
    }
}

fn row_to_article(row: &rusqlite::Row) -> rusqlite::Result<Article> {
    let id: String = row.get(0)?;
    let provider_id: String = row.get(1)?;
    let original_url: String = row.get(2)?;
    let canonical_url: String = row.get(3)?;
    let url_hash: String = row.get(4)?;
    let title: String = row.get(5)?;
    let byline: Option<String> = row.get(6)?;
    let summary: Option<String> = row.get(7)?;
    let content_text: String = row.get(8)?;
    let content_html: Option<String> = row.get(9)?;

    let pub_str: Option<String> = row.get(10)?;
    let published_at = pub_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));

    let scrap_str: String = row.get(11)?;
    let scraped_at = DateTime::parse_from_rfc3339(&scrap_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let top_image: Option<String> = row.get(12)?;
    let tags_json: String = row.get(13)?;
    let language: Option<String> = row.get(14)?;
    let metadata_json: String = row.get(15)?;

    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let metadata: HashMap<String, String> = serde_json::from_str(&metadata_json).unwrap_or_default();

    Ok(Article {
        id,
        provider_id,
        original_url,
        canonical_url,
        url_hash,
        title,
        byline,
        summary,
        content_text,
        content_html,
        published_at,
        scraped_at,
        top_image,
        tags,
        language,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_storage_deduplication() {
        let storage = SqliteStorage::in_memory().unwrap();

        let article = Article {
            id: "art-1".into(),
            provider_id: "cnbc-id".into(),
            original_url: "https://cnbcindonesia.com/news/1".into(),
            canonical_url: "https://cnbcindonesia.com/news/1".into(),
            url_hash: "hash123".into(),
            title: "IHSG Naik 2%".into(),
            byline: Some("Reporter A".into()),
            summary: Some("Ringkasan".into()),
            content_text: "Konten berita pasar modal...".into(),
            content_html: None,
            published_at: None,
            scraped_at: Utc::now(),
            top_image: None,
            tags: vec!["pasar-modal".into(), "saham".into()],
            language: Some("ID".into()),
            metadata: HashMap::new(),
        };

        // Belum ada di awal
        assert!(!storage.contains_hash("hash123").await.unwrap());

        // Simpan
        storage.save_article(&article).await.unwrap();

        // Sekarang harus sudah ada
        assert!(storage.contains_hash("hash123").await.unwrap());

        // Cek retrieval
        let articles = storage.get_articles(None, 10, 0).await.unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "IHSG Naik 2%");
        assert_eq!(articles[0].tags, vec!["pasar-modal", "saham"]);

        // Simpan ulang dengan hash sama tidak error dan tidak menduplikasi
        storage.save_article(&article).await.unwrap();
        assert_eq!(storage.count_articles(None).await.unwrap(), 1);
    }
}
