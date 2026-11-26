use crate::feed::parse_feed_content;
use crate::limiter::DomainRateLimiter;
use crate::proxy::ProxyPool;
use async_trait::async_trait;
use eay_scraper_core::error::{Result, ScraperError};
use eay_scraper_core::model::FeedItem;
use eay_scraper_core::pipeline::Fetcher;
use rand::seq::SliceRandom;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, info, warn};
use url::Url;

/// Daftar User-Agent modern realistis untuk rotasi
const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.5; rv:127.0) Gecko/20100101 Firefox/127.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:127.0) Gecko/20100101 Firefox/127.0",
];

/// HTTP Fetcher berkinerja tinggi berbasis `reqwest` dengan connection pool,
/// polite rate limiter, proxy rotation, dan evasive headers.
#[derive(Clone)]
pub struct HttpFetcher {
    client: Client,
    proxy_pool: ProxyPool,
    limiter: DomainRateLimiter,
    max_retries: u32,
}

impl HttpFetcher {
    pub fn new(proxy_pool: ProxyPool, default_delay: Duration) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            proxy_pool,
            limiter: DomainRateLimiter::new(default_delay),
            max_retries: 3,
        }
    }

    /// Pilih User-Agent acak untuk evasive anti-scraping
    fn random_user_agent(&self) -> &'static str {
        let mut rng = rand::thread_rng();
        USER_AGENTS.choose(&mut rng).unwrap_or(&USER_AGENTS[0])
    }

    /// Buat default browser headers
    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(self.random_user_agent()),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
            ),
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7"),
        );
        headers
    }
}

#[async_trait]
impl Fetcher for HttpFetcher {
    async fn fetch_text(&self, url: &str) -> Result<String> {
        let parsed_url = Url::parse(url)?;
        let domain = parsed_url.host_str().unwrap_or("unknown");

        // Hormati rate limit per domain
        self.limiter.wait(domain, None).await;

        let mut attempt = 0;
        let mut last_err = String::new();

        while attempt < self.max_retries {
            attempt += 1;
            debug!("Fetching URL: {} (attempt {}/{})", url, attempt, self.max_retries);

            let req_builder = self.client.get(url).headers(self.build_headers());
            if let Some(proxy_addr) = self.proxy_pool.next_proxy() {
                debug!("Using proxy {} for {}", proxy_addr, url);
            }

            match req_builder.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let text = resp.text().await.map_err(|e| ScraperError::Network(e.to_string()))?;
                        return Ok(text);
                    } else if status.as_u16() == 429 {
                        warn!("Rate limited (429) on {}, waiting backoff...", domain);
                        tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                        last_err = format!("HTTP 429 Rate limited on {}", domain);
                    } else {
                        return Err(ScraperError::HttpStatus {
                            code: status.as_u16(),
                            message: format!("HTTP request failed with status: {}", status),
                        });
                    }
                }
                Err(err) => {
                    warn!("Request error on {}: {:?}, retrying...", url, err);
                    tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                    last_err = err.to_string();
                }
            }
        }

        Err(ScraperError::Network(format!(
            "Failed after {} attempts to fetch {}: {}",
            self.max_retries, url, last_err
        )))
    }

    async fn fetch_feed_items(&self, provider_id: &str, feed_url: &str) -> Result<Vec<FeedItem>> {
        info!("Fetching feed for [{}] from {}", provider_id, feed_url);
        let content = self.fetch_text(feed_url).await?;
        parse_feed_content(provider_id, content.as_bytes())
    }
}
