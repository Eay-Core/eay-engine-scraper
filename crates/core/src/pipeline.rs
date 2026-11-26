use crate::error::Result;
use crate::model::{Article, FeedItem};
use crate::recipe::Recipe;
use async_trait::async_trait;

/// Abstraksi penarikan data dari jaringan internet
#[async_trait]
pub trait Fetcher: Send + Sync {
    /// Mengambil konten teks mentah (HTML atau XML) dari sebuah URL
    async fn fetch_text(&self, url: &str) -> Result<String>;

    /// Menemukan artikel baru dari Feed (RSS / Atom)
    async fn fetch_feed_items(&self, provider_id: &str, feed_url: &str) -> Result<Vec<FeedItem>>;
}

/// Abstraksi ekstraksi data dari HTML menjadi entitas `Article`
pub trait Extractor: Send + Sync {
    /// Mengekstrak artikel menggunakan recipe spesifik atau fallback
    fn extract(&self, html: &str, url: &str, recipe: Option<&Recipe>) -> Result<Article>;
}

/// Abstraksi penyimpanan dan deduplikasi artikel
#[async_trait]
pub trait StorageSink: Send + Sync {
    /// Memeriksa apakah URL hash sudah tersimpan sebelumnya
    async fn contains_hash(&self, url_hash: &str) -> Result<bool>;

    /// Menyimpan satu artikel yang berhasil di-scrape
    async fn save_article(&self, article: &Article) -> Result<()>;

    /// Mengambil daftar artikel dengan paginasi dan filter opsional
    async fn get_articles(
        &self,
        provider_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Article>>;

    /// Mengambil artikel berdasarkan ID unik
    async fn get_article_by_id(&self, id: &str) -> Result<Option<Article>>;

    /// Mengambil total hitungan artikel tersimpan
    async fn count_articles(&self, provider_id: Option<&str>) -> Result<usize>;
}
