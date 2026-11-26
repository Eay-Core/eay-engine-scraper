use chrono::{DateTime, Utc};
use eay_scraper_core::error::{Result, ScraperError};
use eay_scraper_core::model::FeedItem;
use eay_scraper_core::url_norm::{hash_canonical_url, normalize_url};
use rss::Channel;
use std::io::Cursor;
use tracing::warn;

/// Parser feed serbaguna yang mampu membaca RSS 2.0 dan Atom feed
pub fn parse_feed_content(provider_id: &str, content_bytes: &[u8]) -> Result<Vec<FeedItem>> {
    // 1. Coba parse sebagai RSS 2.0
    if let Ok(channel) = Channel::read_from(Cursor::new(content_bytes)) {
        let mut items = Vec::new();
        for item in channel.items() {
            let raw_link = item
                .link()
                .or_else(|| item.guid().map(|g| g.value()))
                .unwrap_or_default();

            if raw_link.is_empty() {
                continue;
            }

            let canonical_url = match normalize_url(raw_link) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let title = item.title().unwrap_or("Untitled").trim().to_string();
            let summary = item.description().map(|s| s.trim().to_string());
            let author = item.author().map(|s| s.trim().to_string());
            let published_at = item
                .pub_date()
                .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let url_hash = hash_canonical_url(&canonical_url);

            items.push(FeedItem {
                provider_id: provider_id.to_string(),
                title,
                original_url: raw_link.to_string(),
                canonical_url,
                url_hash,
                published_at,
                summary,
                author,
            });
        }
        return Ok(items);
    }

    // 2. Coba parse sebagai Atom Feed
    if let Ok(feed) = atom_syndication::Feed::read_from(Cursor::new(content_bytes)) {
        let mut items = Vec::new();
        for entry in feed.entries() {
            let raw_link = entry
                .links()
                .first()
                .map(|l| l.href().to_string())
                .unwrap_or_else(|| entry.id().to_string());

            if raw_link.is_empty() {
                continue;
            }

            let canonical_url = match normalize_url(&raw_link) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let title = entry.title().as_str().trim().to_string();
            let summary = entry.summary().map(|s| s.as_str().trim().to_string());
            let author = entry.authors().first().map(|a| a.name().to_string());
            let published_at = Some(entry.updated().with_timezone(&Utc));
            let url_hash = hash_canonical_url(&canonical_url);

            items.push(FeedItem {
                provider_id: provider_id.to_string(),
                title,
                original_url: raw_link,
                canonical_url,
                url_hash,
                published_at,
                summary,
                author,
            });
        }
        return Ok(items);
    }

    warn!("Failed to parse feed as RSS or Atom");
    Err(ScraperError::Parse("Unrecognized feed format (neither RSS nor Atom)".into()))
}
