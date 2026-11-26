# ADR 0001: Modular Workspace Architecture for Multi-Context Scraper Engine

- **Status**: Accepted
- **Date**: 2026-11-26
- **Context**: `eay-engine-scraper` dirancang untuk mengumpulkan berita dan konten internet dari berbagai provider lokal (CNBC Indonesia, Liputan6, Kompas, dll) maupun global (Reuters, BBC, dll). Engine ini harus fleksibel: dapat dijalankan sebagai daemon mandiri di server (self-hosted), di-embed ke aplikasi desktop (Tauri / Flutter) sebagai library lokal, dihubungkan antar-plugin, serta mendukung antarmuka REST API.
- **Decision**: Kami memisahkan arsitektur ke dalam Cargo Workspace modular:
  1. `crates/core`: Zero-heavy dependency crate yang mendefinisikan domain model (`Article`, `Recipe`, `Feed`), traits (`Fetcher`, `Extractor`, `Sink`, `Scheduler`), URL normalization, dan Readability heuristic dasar.
  2. `crates/fetcher`: Implementasi async HTTP fetcher menggunakan `reqwest`, connection pooling, dynamic proxy pool rotator, domain rate limiter (leaky-bucket / token-bucket via governor), dan feature flag opsional `headless` (Chromium CDP).
  3. `crates/storage-sqlite`: Implementasi `Sink` dan deduplication registry berbasis SQLite (`rusqlite` / `sqlx-sqlite`) dengan migrasi otomatis.
  4. `crates/daemon`: Engine runtime dengan continuous scheduler, config watcher (hot-reload recipes), dan REST API server (`axum`) untuk kontrol & inspeksi.
  5. `crates/cli`: Binary antarmuka baris perintah (`clap`) untuk eksekusi manual, dry-run scraping provider, dan inspeksi SQLite.
- **Consequences**:
  - Positif: `core` sangat ringan dan dapat di-reuse oleh proyek desktop (Tauri) atau integrasi plugin tanpa menarik dependency web server atau chromium.
  - Positif: Pemisahan concern yang bersih, testing terisolasi per crate.
  - Negatif: Memerlukan setup workspace Cargo dengan dependencies shared via workspace inheritance.
