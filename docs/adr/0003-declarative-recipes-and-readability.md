# ADR 0003: Declarative YAML Recipes with Fallback Heuristic Extractor

- **Status**: Accepted
- **Date**: 2026-11-26
- **Context**: Portal berita sering mengubah format layout HTML mereka. Jika selector di-hardcode ke dalam kode Rust, setiap perubahan layout memerlukan kompilasi dan deployment ulang aplikasi. Di sisi lain, untuk website umum tanpa recipe khusus, engine tetap harus mampu mengekstrak konten artikel.
- **Decision**:
  1. **Declarative Recipe (YAML)**: Aturan scraping per provider (RSS endpoint, target pattern, selector title, selector content, selector author, selectors to drop) disimpan dalam file YAML di direktori `recipes/`.
  2. **Hot-reloading**: Engine memantau folder `recipes/` atau membaca saat startup sehingga pengguna/admin bisa menambahkan provider baru (misal `kompas.yaml`, `detik.yaml`, `bloomberg.yaml`) secara instan tanpa compile ulang.
  3. **Auto-Readability Fallback**: Bila selector CSS tidak cocok atau recipe tidak terdefinisi, sistem secara otomatis mengekstrak konten menggunakan algoritma heuristik berbasis kepadatan teks (Text Density & Readability scoring).
- **Consequences**:
  - Penambahan dan penyesuaian provider sangat fleksibel dan dapat dikontribusikan oleh komunitas/pengguna tanpa keahlian Rust.
  - Readability extractor memberikan jaminan ekstraksi bahkan saat layout provider berubah sebelum recipe diperbarui.
