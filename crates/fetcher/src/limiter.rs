use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// Per-domain polite rate limiter
/// Mencegah penyerbuan request ke satu domain host yang sama dalam interval yang terlalu rapat
#[derive(Debug, Clone)]
pub struct DomainRateLimiter {
    last_requests: Arc<Mutex<HashMap<String, Instant>>>,
    default_delay: Duration,
}

impl DomainRateLimiter {
    pub fn new(default_delay: Duration) -> Self {
        Self {
            last_requests: Arc::new(Mutex::new(HashMap::new())),
            default_delay,
        }
    }

    /// Tunggu jika request terakhir ke domain ini belum melewati `domain_delay`
    pub async fn wait(&self, domain: &str, custom_delay: Option<Duration>) {
        let delay = custom_delay.unwrap_or(self.default_delay);
        let mut map = self.last_requests.lock().await;

        let now = Instant::now();
        if let Some(last_time) = map.get(domain) {
            let elapsed = now.duration_since(*last_time);
            if elapsed < delay {
                let sleep_duration = delay - elapsed;
                drop(map); // Release lock sebelum await sleep
                tokio::time::sleep(sleep_duration).await;

                // Update kembali setelah bangun
                let mut map = self.last_requests.lock().await;
                map.insert(domain.to_string(), Instant::now());
                return;
            }
        }

        map.insert(domain.to_string(), now);
    }
}
