use sha2::{Digest, Sha256};
use url::Url;

/// Daftar query params umum yang dipakai untuk tracking / analitik dan harus dieliminasi
const TRACKING_PARAMS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    "fbclid",
    "gclid",
    "gclsrc",
    "dclid",
    "zanpid",
    "msclkid",
    "mc_cid",
    "mc_eid",
    "_ga",
    "_gl",
    "ref",
    "ref_src",
    "source",
    "ocid",
    "ncid",
];

/// Normalisasi URL ke bentuk standar yang bersih dan deterministik:
/// 1. Parse dan validasi URL
/// 2. Lowercase scheme dan host
/// 3. Buang URL fragment `#...`
/// 4. Buang semua query params tracking (UTM, Facebook clid, Google click id, dll)
/// 5. Urutkan sisa query params agar deterministik
/// 6. Hapus trailing slash bila bukan root
pub fn normalize_url(raw_url: &str) -> crate::error::Result<String> {
    let trimmed = raw_url.trim();
    let mut parsed = Url::parse(trimmed)?;

    // Scheme & host sudah otomatis di-lowercase oleh Url::parse
    parsed.set_fragment(None);

    // Filter tracking params & sort query pairs
    let filtered_pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| !TRACKING_PARAMS.contains(&k.to_ascii_lowercase().as_str()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if filtered_pairs.is_empty() {
        parsed.set_query(None);
    } else {
        let mut sorted = filtered_pairs;
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        let mut serializer = form_urlencoded::Serializer::new(String::new());
        for (k, v) in sorted {
            serializer.append_pair(&k, &v);
        }
        parsed.set_query(Some(&serializer.finish()));
    }

    let mut result = parsed.to_string();

    // Hapus trailing slash jika bukan path root "https://domain.com/"
    if result.ends_with('/') && parsed.path() != "/" && parsed.path() != "" {
        result.pop();
    }

    Ok(result)
}

/// Menghitung SHA256 hex string dari URL kanonikal untuk indexing O(1) di database
pub fn hash_canonical_url(canonical_url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_url.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_strips_tracking() {
        let raw = "https://www.cnbcindonesia.com/market/20261125-17-695000/saham-ihsg-melejit?utm_source=twitter&utm_medium=social&utm_campaign=breaking#komentar";
        let norm = normalize_url(raw).unwrap();
        assert_eq!(
            norm,
            "https://www.cnbcindonesia.com/market/20261125-17-695000/saham-ihsg-melejit"
        );

        let hash1 = hash_canonical_url(&norm);
        let hash2 = hash_canonical_url(
            "https://www.cnbcindonesia.com/market/20261125-17-695000/saham-ihsg-melejit",
        );
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_normalize_preserves_legitimate_query() {
        let raw = "https://example.com/search?q=rust+scraper&page=2&fbclid=IwAR23123";
        let norm = normalize_url(raw).unwrap();
        assert_eq!(norm, "https://example.com/search?page=2&q=rust+scraper");
    }
}
