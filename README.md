# EAY Engine Scraper (`eay-engine-scraper`)

Engine modular berkecepatan tinggi dan berdaya tahan tinggi yang dibangun dengan **Rust & Tokio** untuk mengumpulkan, menormalisasi, dan mengekstraksi berita serta konten web apa pun di internet (multi-provider lokal seperti **CNBC Indonesia**, **Liputan6**, maupun media global seperti **BBC News**).

---

## 🚀 Fitur Utama

- **Modular Cargo Workspace**:
  - `crates/core`: Model domain (`Article`, `Recipe`, `FeedItem`), normalisasi URL, deduplikasi SHA256, dan Readability heuristic extractor.
  - `crates/fetcher`: HTTP Fetcher berbasis `reqwest` dengan pool koneksi, proxy pool rotator (HTTP/SOCKS5), per-domain rate limiter, dan evasive rotating User-Agents.
  - `crates/storage-sqlite`: Storage sink thread-safe dengan deduplikasi O(1) B-Tree SQLite (WAL mode, memory cache).
  - `crates/daemon`: Engine runtime dengan background continuous scheduler, config hot-reloader, dan REST API server (`axum`).
  - `crates/cli`: Command Line Interface (`eay-scraper`) untuk eksekusi manual, dry-run provider, dan inspeksi database.
- **Deduplikasi 2 Lapis**:
  1. *Canonical URL Normalizer*: Membersihkan tracking UTM params (`utm_*`, `fbclid`, `gclid`, dll) dan menstandarkan format URL.
  2. *URL Hash Store*: Hash SHA256 di-index unik pada SQLite untuk mengeliminasi fetch berulang (*short-circuit*).
- **Declarative Hot-Reloadable Recipes (YAML)**:
  - Menambah atau memperbarui selektor provider baru cukup meletakkan file `.yaml` di folder `recipes/` tanpa perlu compile ulang.
- **Auto-Readability Fallback**:
  - Jika selektor CSS tidak cocok atau recipe belum dibuat, engine otomatis membedah struktur HTML dan mengekstrak judul, gambar utama, dan badan artikel menggunakan algoritma densitas teks.
- **REST Control Plane (`axum`)**:
  - `/health`: Cek status, jumlah artikel, dan loaded recipes.
  - `/api/v1/articles`: List artikel tersimpan dengan pagination & filter provider.
  - `/api/v1/articles/:id`: Detail artikel lengkap.
  - `/api/v1/recipes`: Daftar recipe aktif.
  - `/api/v1/recipes/reload`: Hot-reload folder recipes saat runtime.
  - `/api/v1/scrape/url`: Eksekusi scraping instan untuk URL tertentu.
  - `/api/v1/scrape/provider/:id`: Trigger scraping batch provider via API.

---

## 🛠️ Cara Menjalankan

### 1. Build Proyek
```bash
cargo build --release
```

### 2. Tampilkan Daftar Provider / Recipe
```bash
./target/release/eay-scraper list-recipes
```

### 3. Eksekusi Scrape Satu Provider (Contoh: CNBC Indonesia)
```bash
./target/release/eay-scraper scrape-provider cnbc-indonesia
```

### 4. Scrape Satu URL Berita Spesifik (Contoh: Liputan6 / BBC)
```bash
./target/release/eay-scraper scrape-url "https://www.liputan6.com/news/read/8343373/mako-korpasgat-tni-au-di-pancoran-terbakar" --recipe liputan6
```

### 5. Tampilkan Artikel yang Tersimpan di SQLite
```bash
./target/release/eay-scraper list-articles --limit 10
```

### 6. Jalankan Daemon Mode (Continuous Background Scraping + HTTP Server)
```bash
./target/release/eay-scraper daemon --host 127.0.0.1 --port 8089 --interval 300
```

---

## 📁 Format Recipe Deklaratif (`recipes/*.yaml`)

Setiap media baru dapat didaftarkan dengan membuat file YAML di direktori `recipes/`:

```yaml
id: "cnbc-indonesia"
name: "CNBC Indonesia"
base_url: "https://www.cnbcindonesia.com"
category: "business"
country: "ID"
rate_limit_ms: 1000
requires_js: false
feeds:
  - "https://www.cnbcindonesia.com/market/rss"
  - "https://www.cnbcindonesia.com/news/rss"
selectors:
  title: "h1.detail_title, h1"
  content: "div.detail_text, article"
  author: "div.author, span.name"
  published_at: "div.date"
  summary: "meta[name='description']"
  top_image: "meta[property='og:image']"
  tags: "div.detail__tag a"
  drop_elements:
    - "script"
    - "style"
    - ".baca-juga"
    - ".ads-container"
```

---

## 📚 Domain & Architecture Docs
- [GLOSSARY.md](./GLOSSARY.md): Definisi konsep domain (Provider, Feed, Recipe, Article, Deduplication, Sink, Extractor).
- [ADR 0001: Modular Workspace Architecture](./docs/adr/0001-modular-workspace-architecture.md)
- [ADR 0002: Deduplication and Canonical URL](./docs/adr/0002-deduplication-and-canonical-url.md)
- [ADR 0003: Declarative YAML Recipes and Readability](./docs/adr/0003-declarative-recipes-and-readability.md)
