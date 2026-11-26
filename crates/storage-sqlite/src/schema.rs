pub const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS articles (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    original_url TEXT NOT NULL,
    canonical_url TEXT NOT NULL,
    url_hash TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    byline TEXT,
    summary TEXT,
    content_text TEXT NOT NULL,
    content_html TEXT,
    published_at TEXT,
    scraped_at TEXT NOT NULL,
    top_image TEXT,
    tags_json TEXT NOT NULL DEFAULT '[]',
    language TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_articles_provider ON articles(provider_id);
CREATE INDEX IF NOT EXISTS idx_articles_scraped_at ON articles(scraped_at DESC);
CREATE INDEX IF NOT EXISTS idx_articles_url_hash ON articles(url_hash);

CREATE TABLE IF NOT EXISTS scrape_jobs (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    total_discovered INTEGER DEFAULT 0,
    new_saved INTEGER DEFAULT 0,
    error_message TEXT
);
"#;
