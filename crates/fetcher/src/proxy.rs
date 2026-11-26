use rand::seq::SliceRandom;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Proxy pool rotator mendukung HTTP, HTTPS, dan SOCKS5 proxy
#[derive(Debug, Clone, Default)]
pub struct ProxyPool {
    proxies: Arc<Vec<String>>,
    counter: Arc<AtomicUsize>,
}

impl ProxyPool {
    /// Membuat pool kosong (koneksi direct)
    pub fn new() -> Self {
        Self {
            proxies: Arc::new(Vec::new()),
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Membuat pool dari daftar URL proxy
    pub fn from_proxies(proxies: Vec<String>) -> Self {
        Self {
            proxies: Arc::new(proxies),
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Ambil proxy berikutnya secara Round-Robin
    pub fn next_proxy(&self) -> Option<String> {
        if self.proxies.is_empty() {
            return None;
        }
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.proxies.len();
        Some(self.proxies[idx].clone())
    }

    /// Ambil proxy secara acak
    pub fn random_proxy(&self) -> Option<String> {
        let mut rng = rand::thread_rng();
        self.proxies.choose(&mut rng).cloned()
    }

    /// Jumlah proxy terdaftar
    pub fn len(&self) -> usize {
        self.proxies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.proxies.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_pool_round_robin() {
        let pool = ProxyPool::from_proxies(vec![
            "http://1.1.1.1:8080".into(),
            "http://2.2.2.2:8080".into(),
        ]);
        assert_eq!(pool.next_proxy(), Some("http://1.1.1.1:8080".into()));
        assert_eq!(pool.next_proxy(), Some("http://2.2.2.2:8080".into()));
        assert_eq!(pool.next_proxy(), Some("http://1.1.1.1:8080".into()));
    }
}
