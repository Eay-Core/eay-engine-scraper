# Glossary: eay-engine-scraper

Definisi istilah domain untuk sistem penelusuran, pengumpulan, dan normalisasi konten (news & web scraping engine). Dokumen ini berfokus murni pada konsep domain tanpa detail teknis implementasi tertentu.

---

### Provider
Entitas media, portal berita, atau sumber web pihak ketiga (misalnya CNBC Indonesia, Liputan6, Reuters) yang mempublikasikan konten secara berkala.

### Feed
Koleksi terstruktur metadata publikasi berkala dari sebuah Provider (seperti RSS 2.0, Atom feed, atau JSON Feed) yang memuat daftar tautan artikel terbaru.

### Recipe
Spesifikasi aturan deklaratif yang memetakan struktur situs web Provider ke model data kanonikal. Recipe mendefinisikan titik awal penelusuran (Feed URL / crawl targets), CSS selector untuk konten (judul, badan artikel, tanggal terbit, pengarang, tag), serta aturan sanitasi elemen yang perlu dibuang (iklan, rekomendasi baca juga, widget).

### Article
Representasi data berita atau tulisan yang telah dinormalisasi dari sebuah Provider, mencakup judul, teks isi bersih (plain text & markdown/HTML bersih), tautan sumber kanonikal, tanggal publikasi, metadata penulis, dan daftar tag/topik.

### Canonical URL
Bentuk standar dan unik dari sebuah URL setelah membersihkan parameter pelacak (*query tracking* seperti `utm_*`, `fbclid`, sesi, dan token referral) serta menyelesaikan redirect, yang digunakan sebagai identitas primer dokumen.

### Content Fingerprint
Nilai ringkasan hash atau identifikasi khas dari konten artikel atau URL kanonikal yang digunakan untuk mengenali duplikasi artikel di seluruh siklus scraping.

### Deduplication
Proses verifikasi apakah sebuah artikel sudah pernah diambil dan diproses sebelumnya guna mencegah pengambilan berulang, penghematan bandwidth, serta mencegah redundansi data pada penyimpanan.

### Ingestion Pipeline
Rangkaian tahapan terurut mulai dari penemuan URL (*discovery*), pengunduhan konten mentah (*fetching*), ekstraksi data berstruktur (*extracting*), normalisasi dan sanitasi (*normalizing*), hingga persistensi (*storage/sink*).

### Fetcher
Komponen yang bertugas mengambil muatan mentah (raw bytes / HTML / XML) dari target URL di internet dengan mematuhi etika penelusuran (rate limiting) dan mekanisme ketahanan jaringan (retry, backoff, proxy).

### Extractor
Komponen yang membedah muatan mentah menjadi struktur Article berdasarkan aturan Recipe spesifik atau menggunakan heuristik keterbacaan (*readability algorithm*) ketika Recipe spesifik tidak tersedia.

### Sink
Muara atau saluran tujuan pengiriman data Article yang telah diproses dan tervalidasi (seperti database lokal, message queue, atau plugin API).
