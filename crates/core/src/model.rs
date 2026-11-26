use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata dasar tentang provider sumber berita
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub country: Option<String>,
    pub category: Option<String>,
}

/// Item feed yang ditemukan dari RSS, Atom, sitemap, atau listing page
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedItem {
    pub provider_id: String,
    pub title: String,
    pub original_url: String,
    pub canonical_url: String,
    pub url_hash: String,
    pub published_at: Option<DateTime<Utc>>,
    pub summary: Option<String>,
    pub author: Option<String>,
}

/// Dokumen artikel lengkap yang telah berhasil di-fetch, di-ekstrak, dan dinormalisasi
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Article {
    pub id: String,
    pub provider_id: String,
    pub original_url: String,
    pub canonical_url: String,
    pub url_hash: String,
    pub title: String,
    pub byline: Option<String>,
    pub summary: Option<String>,
    pub content_text: String,
    pub content_html: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub scraped_at: DateTime<Utc>,
    pub top_image: Option<String>,
    pub tags: Vec<String>,
    pub language: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Batch hasil penelusuran satu siklus
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestionBatch {
    pub provider_id: String,
    pub total_discovered: usize,
    pub new_items: usize,
    pub skipped_existing: usize,
    pub failed_items: usize,
    pub articles: Vec<Article>,
}
