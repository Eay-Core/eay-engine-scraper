use anyhow::Result;
use clap::{Parser, Subcommand};
use eay_scraper_core::pipeline::StorageSink;
use eay_scraper_daemon::api::run_server;
use eay_scraper_daemon::config::DaemonConfig;
use eay_scraper_daemon::engine::EngineRuntime;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(
    name = "eay-scraper",
    author = "EAY Corp <dev@eay.corp>",
    version = "0.1.0",
    about = "High-performance modular news and web scraper engine"
)]
struct Cli {
    #[arg(short, long, default_value = "scraper.sqlite")]
    db: String,

    #[arg(short, long, default_value = "recipes")]
    recipes: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Jalankan daemon service secara continuous (background loop + HTTP control plane)
    Daemon {
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        #[arg(short, long, default_value_t = 8089)]
        port: u16,

        #[arg(short, long, default_value_t = 300)]
        interval: u64,
    },

    /// Scrape satu URL secara langsung untuk inspeksi/debugging
    ScrapeUrl {
        url: String,

        #[arg(short, long)]
        recipe: Option<String>,
    },

    /// Jalankan scraping satu putaran untuk satu provider tertentu (misal: cnbc-indonesia)
    ScrapeProvider {
        provider_id: String,
    },

    /// Tampilkan daftar semua recipe yang terdeteksi
    ListRecipes,

    /// Tambahkan custom recipe baru via CLI
    AddRecipe {
        #[arg(short, long)]
        id: String,

        #[arg(short, long)]
        name: String,

        #[arg(short, long)]
        base_url: String,

        #[arg(short, long)]
        feed: Vec<String>,

        #[arg(short, long, default_value = "custom")]
        category: String,

        #[arg(short = 'C', long, default_value = "GLOBAL")]
        country: String,
    },

    /// Tampilkan artikel yang tersimpan di SQLite
    ListArticles {
        #[arg(short, long)]
        provider: Option<String>,

        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,eay_scraper=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    let mut config = DaemonConfig::default();
    config.db_path = cli.db;
    config.recipes_dir = cli.recipes;

    let runtime = Arc::new(EngineRuntime::new(config.clone()).await?);

    match cli.command {
        Commands::Daemon { host, port, interval } => {
            info!("Starting EAY Engine Scraper Daemon on {}:{}", host, port);
            let mut daemon_cfg = config;
            daemon_cfg.http_host = host.clone();
            daemon_cfg.http_port = port;
            daemon_cfg.scrape_interval_secs = interval;

            let rt_clone = Arc::clone(&runtime);
            tokio::spawn(async move {
                rt_clone.start_background_loop().await;
            });

            run_server(runtime, &host, port).await?;
        }

        Commands::ScrapeUrl { url, recipe } => {
            println!("🔍 Fetching & extracting URL: {}", url);
            let article = runtime.scrape_single_url(&url, recipe.as_deref()).await?;
            println!("\n✅ Scraped Article:");
            println!("   ID:       {}", article.id);
            println!("   Title:    {}", article.title);
            println!("   Provider: {}", article.provider_id);
            println!("   URL:      {}", article.canonical_url);
            println!("   Hash:     {}", article.url_hash);
            println!("   Byline:   {:?}", article.byline);
            println!("   Summary:  {:?}", article.summary);
            println!("   Tags:     {:?}", article.tags);
            println!("   Content:  {} chars", article.content_text.len());
            println!("   Preview:  {}...", article.content_text.chars().take(200).collect::<String>());
        }

        Commands::ScrapeProvider { provider_id } => {
            println!("🚀 Running scrape batch for provider: [{}]", provider_id);
            if let Some(recipe) = runtime.recipes().get_recipe(&provider_id).await {
                let batch = runtime.run_provider(&recipe).await?;
                println!("\n📊 Batch Summary for [{}]:", provider_id);
                println!("   Total Discovered: {}", batch.total_discovered);
                println!("   Skipped (Exists): {}", batch.skipped_existing);
                println!("   New Saved:        {}", batch.new_items);
                println!("   Failed:           {}", batch.failed_items);
            } else {
                eprintln!("❌ Error: Recipe [{}] not found in recipes directory", provider_id);
            }
        }

        Commands::ListRecipes => {
            let recipes = runtime.recipes().list_recipes().await;
            println!("📋 Registered Recipes ({}):", recipes.len());
            for r in recipes {
                println!("  - [{}] {} (Base: {}, Feeds: {})", r.id, r.name, r.base_url, r.feeds.len());
            }
        }

        Commands::AddRecipe {
            id,
            name,
            base_url,
            feed,
            category,
            country,
        } => {
            let recipe = eay_scraper_core::recipe::Recipe {
                id: id.clone(),
                name: name.clone(),
                base_url: base_url.clone(),
                category: Some(category),
                country: Some(country),
                rate_limit_ms: 1000,
                requires_js: false,
                feeds: if feed.is_empty() {
                    vec![format!("{}/rss", base_url.trim_end_matches('/'))]
                } else {
                    feed
                },
                url_patterns: vec![format!("{}/.*", base_url.trim_end_matches('/'))],
                selectors: eay_scraper_core::recipe::RecipeSelectors::default(),
            };

            runtime.recipes().add_or_update_recipe(recipe).await?;
            println!("✅ Successfully created and registered custom recipe [{}] ({})", id, name);
        }

        Commands::ListArticles { provider, limit } => {
            let total = runtime.storage().count_articles(provider.as_deref()).await?;
            let articles = runtime.storage().get_articles(provider.as_deref(), limit, 0).await?;

            println!("📰 Stored Articles (Showing {} of {} total):", articles.len(), total);
            for (idx, a) in articles.iter().enumerate() {
                println!(
                    "  {}. [{}] {} ({})",
                    idx + 1,
                    a.provider_id,
                    a.title,
                    a.canonical_url
                );
            }
        }
    }

    Ok(())
}
