# ADR 0002: Deduplication via Canonical URL Normalization and SQLite Fingerprint Store

- **Status**: Accepted
- **Date**: 2026-11-26
- **Context**: Portal berita sering menggunakan query parameters untuk analitik (`utm_source`, `utm_medium`, `fbclid`, `ref`, dll) dan RSS feed sering mempublikasikan ulang URL yang sama setiap beberapa jam. Engine scraping harus mencegah redundansi fetch dan duplikasi penyimpanan.
- **Decision**: Kami menerapkan deduplikasi dua tingkat:
  1. **Canonical URL Normalization**: Setiap URL yang ditemukan (dari RSS atau crawler) dinormalisasi secara deterministik:
     - Protokol & hostname di-lowercase.
     - Query parameters yang masuk blacklist analitik (`utm_*`, `fbclid`, `gclid`, `ref`, `_ga`, `ocid`, dsb) dieliminasi.
     - Trailing slash distandarisasi dan fragment identifier (`#...`) dibuang.
  2. **SHA256 Fingerprint di SQLite**:
     - Hash SHA256 dari URL kanonikal disimpan dengan index unik di SQLite (`articles.url_hash` dan `fingerprints`).
     - Sebelum melakukan deep fetch (mengambil body artikel lengkap), engine memeriksa apakah `url_hash` sudah ada di database. Jika sudah ada, pengunduhan diabaikan (*short-circuit*).
- **Consequences**:
  - Menghemat bandwidth dan mencegah throttling / rate-limit dari media.
  - Waktu simpan O(1) dengan indeks B-Tree SQLite.
