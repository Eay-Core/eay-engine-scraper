use serde::{Deserialize, Serialize};

/// Recipe deklaratif untuk memetakan situs web atau media tertentu
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recipe {
    /// Identifier unik provider (misal: "cnbc-id", "liputan6", "reuters")
    pub id: String,

    /// Nama manusiawi provider (misal: "CNBC Indonesia")
    pub name: String,

    /// Domain dasar website
    pub base_url: String,

    /// Kategori default (misal: "business", "news", "technology")
    #[serde(default)]
    pub category: Option<String>,

    /// Negara asal (misal: "ID", "US", "GLOBAL")
    #[serde(default)]
    pub country: Option<String>,

    /// Daftar endpoint RSS/Atom feeds untuk auto-discovery
    #[serde(default)]
    pub feeds: Vec<String>,

    /// Pola URL artikel (regex) yang valid untuk provider ini
    #[serde(default)]
    pub url_patterns: Vec<String>,

    /// Pengaturan selector CSS untuk ekstraksi artikel
    pub selectors: RecipeSelectors,

    /// Delay minimum antar-request ke domain ini (dalam milidetik)
    #[serde(default = "default_rate_limit_ms")]
    pub rate_limit_ms: u64,

    /// Apakah provider ini membutuhkan JS rendering headless browser
    #[serde(default)]
    pub requires_js: bool,
}

fn default_rate_limit_ms() -> u64 {
    1000 // Default 1 detik per domain
}

/// Kumpulan selector CSS yang digunakan oleh engine ekstraksi
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RecipeSelectors {
    /// CSS selector untuk judul artikel
    #[serde(default)]
    pub title: Option<String>,

    /// CSS selector untuk kontainer isi artikel
    #[serde(default)]
    pub content: Option<String>,

    /// CSS selector untuk penulis / author
    #[serde(default)]
    pub author: Option<String>,

    /// CSS selector untuk tanggal publikasi
    #[serde(default)]
    pub published_at: Option<String>,

    /// CSS selector untuk ringkasan / teaser
    #[serde(default)]
    pub summary: Option<String>,

    /// CSS selector untuk featured / top image
    #[serde(default)]
    pub top_image: Option<String>,

    /// CSS selector untuk tag / kategori artikel
    #[serde(default)]
    pub tags: Option<String>,

    /// Elemen-elemen pengganggu di dalam content yang wajib dibersihkan
    /// (misal: "script, style, .baca-juga, .ads-box, .social-share")
    #[serde(default)]
    pub drop_elements: Vec<String>,
}

impl Recipe {
    /// Load recipe dari string YAML
    pub fn from_yaml(yaml_str: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_str)
    }

    /// Load recipe dari file YAML
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content).map_err(|e| crate::error::ScraperError::Recipe(e.to_string()))
    }
}
