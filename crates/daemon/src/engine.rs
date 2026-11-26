use crate::config::DaemonConfig;
use crate::watcher::RecipeRegistry;
use chrono::Utc;
use eay_scraper_core::error::Result;
use eay_scraper_core::extractor::DefaultExtractor;
use eay_scraper_core::model::{Article, IngestionBatch};
use eay_scraper_core::pipeline::{Extractor, Fetcher, StorageSink};
use eay_scraper_core::recipe::Recipe;
use eay_scraper_core::url_norm::normalize_url;
use eay_scraper_fetcher::{HttpFetcher, ProxyPool};
use eay_scraper_storage_sqlite::SqliteStorage;
use futures::future::join_all;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// Orchestrator utama scraping runtime yang menjalankan discovery, deduplikasi,
/// fetching, extracting, dan penyimpanan
#[derive(Clone)]
pub struct EngineRuntime {
    config: DaemonConfig,
    recipes: RecipeRegistry,
    fetcher: Arc<HttpFetcher>,
    extractor: Arc<DefaultExtractor>,
    storage: SqliteStorage,
}

impl EngineRuntime {
    pub async fn new(config: DaemonConfig) -> Result<Self> {
        let recipes = RecipeRegistry::new(&config.recipes_dir);
        recipes.load_all().await?;

        let storage = SqliteStorage::open(&config.db_path)?;
        let fetcher = Arc::new(HttpFetcher::new(
            ProxyPool::new(),
            Duration::from_millis(800),
        ));
        let extractor = Arc::new(DefaultExtractor::new());

        Ok(Self {
            config,
            recipes,
            fetcher,
            extractor,
            storage,
        })
    }

    pub fn storage(&self) -> &SqliteStorage {
        &self.storage
    }

    pub fn recipes(&self) -> &RecipeRegistry {
        &self.recipes
    }

    pub fn config(&self) -> &DaemonConfig {
        &self.config
    }

    /// Jalankan scraping untuk satu URL tunggal secara langsung (bisa dengan atau tanpa recipe)
    pub async fn scrape_single_url(&self, raw_url: &str, recipe_id: Option<&str>) -> Result<Article> {
        let canonical = normalize_url(raw_url)?;
        info!("Single URL scrape: {} (canonical: {})", raw_url, canonical);

        let recipe = if let Some(rid) = recipe_id {
            self.recipes.get_recipe(rid).await
        } else {
            None
        };

        let html = self.fetcher.fetch_text(&canonical).await?;
        let article = self.extractor.extract(&html, &canonical, recipe.as_ref())?;

        // Simpan ke storage
        self.storage.save_article(&article).await?;

        Ok(article)
    }

    /// Jalankan scraping lengkap untuk satu Provider Recipe (Feed discovery -> Deduplikasi -> Fetch -> Simpan)
    pub async fn run_provider(&self, recipe: &Recipe) -> Result<IngestionBatch> {
        info!("--- Starting scrape run for [{}] ({}) ---", recipe.id, recipe.name);

        let mut batch = IngestionBatch {
            provider_id: recipe.id.clone(),
            ..Default::default()
        };

        // 1. Discovery: Kumpulkan feed items dari semua endpoint feed yang terdaftar
        let mut discovered_items = Vec::new();
        for feed_url in &recipe.feeds {
            match self.fetcher.fetch_feed_items(&recipe.id, feed_url).await {
                Ok(items) => {
                    info!("Found {} items from feed {}", items.len(), feed_url);
                    discovered_items.extend(items);
                }
                Err(e) => {
                    warn!("Error fetching feed {}: {}", feed_url, e);
                }
            }
        }

        batch.total_discovered = discovered_items.len();

        // 2. Deduplikasi: saring item yang sudah pernah disimpan di SQLite
        let mut items_to_fetch = Vec::new();
        for item in discovered_items {
            match self.storage.contains_hash(&item.url_hash).await {
                Ok(true) => {
                    batch.skipped_existing += 1;
                }
                Ok(false) => {
                    items_to_fetch.push(item);
                }
                Err(e) => {
                    warn!("Storage check failed for {}: {}", item.url_hash, e);
                    items_to_fetch.push(item);
                }
            }
        }

        // Ambil maksimal 15 item terbaru per batch run agar responsif dan tidak memicu ban
        if items_to_fetch.len() > 15 {
            info!("Capping new items to fetch at 15 for safety");
            items_to_fetch.truncate(15);
        }

        info!(
            "[{}] Discovered: {}, Existing (skipped): {}, New to fetch: {}",
            recipe.id, batch.total_discovered, batch.skipped_existing, items_to_fetch.len()
        );

        // 3. Concurrency-controlled deep fetching
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_tasks));
        let mut tasks = Vec::new();

        for item in items_to_fetch {
            let sem = Arc::clone(&semaphore);
            let fetcher = Arc::clone(&self.fetcher);
            let extractor = Arc::clone(&self.extractor);
            let storage = self.storage.clone();
            let recipe_clone = recipe.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                match fetcher.fetch_text(&item.canonical_url).await {
                    Ok(html) => {
                        match extractor.extract(&html, &item.canonical_url, Some(&recipe_clone)) {
                            Ok(mut article) => {
                                // Pertahankan metadata dari feed jika artikel belum memilikinya
                                if article.published_at.is_none() {
                                    article.published_at = item.published_at;
                                }
                                if article.byline.is_none() {
                                    article.byline = item.author;
                                }
                                if article.summary.is_none() {
                                    article.summary = item.summary;
                                }

                                if let Err(e) = storage.save_article(&article).await {
                                    error!("Failed to save article {}: {}", article.canonical_url, e);
                                    None
                                } else {
                                    Some(article)
                                }
                            }
                            Err(e) => {
                                warn!("Extraction error for {}: {}", item.canonical_url, e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Fetch error for {}: {}", item.canonical_url, e);
                        None
                    }
                }
            }));
        }

        let results = join_all(tasks).await;
        for res in results {
            if let Ok(Some(article)) = res {
                batch.new_items += 1;
                batch.articles.push(article);
            } else {
                batch.failed_items += 1;
            }
        }

        info!(
            "--- Finished [{}] => {} new articles saved, {} failed ---",
            recipe.id, batch.new_items, batch.failed_items
        );

        Ok(batch)
    }

    /// Jalankan scraping untuk semua provider yang terdaftar dalam registry
    pub async fn run_all_providers(&self) -> Vec<IngestionBatch> {
        let recipes = self.recipes.list_recipes().await;
        let mut batches = Vec::new();

        for recipe in recipes {
            match self.run_provider(&recipe).await {
                Ok(b) => batches.push(b),
                Err(e) => {
                    error!("Error running provider {}: {}", recipe.id, e);
                }
            }
        }

        batches
    }

    /// Background loop daemon yang berjalan berkala
    pub async fn start_background_loop(self: Arc<Self>) {
        info!(
            "Starting continuous daemon loop with interval: {} seconds",
            self.config.scrape_interval_secs
        );

        let mut interval = tokio::time::interval(Duration::from_secs(self.config.scrape_interval_secs));
        loop {
            interval.tick().await;
            info!("Daemon tick at {}: running provider sweeps", Utc::now());

            // Reload recipes jika ada file baru di-drop ke folder recipes/
            let _ = self.recipes.load_all().await;

            let _ = self.run_all_providers().await;
        }
    }
}
