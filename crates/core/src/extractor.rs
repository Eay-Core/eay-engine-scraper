use crate::error::{Result, ScraperError};
use crate::model::Article;
use crate::pipeline::Extractor;
use crate::recipe::Recipe;
use crate::url_norm::{hash_canonical_url, normalize_url};
use chrono::Utc;
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashMap;
use uuid::Uuid;

/// Default Content Extractor yang mengimplementasikan deklaratif CSS matching
/// dengan fallback ke heuristik keterbacaan (readability)
#[derive(Debug, Default, Clone)]
pub struct DefaultExtractor;

impl DefaultExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Ekstraksi berbasis Recipe CSS selectors
    fn extract_with_recipe(&self, document: &Html, url: &str, recipe: &Recipe) -> Result<Article> {
        let canonical_url = normalize_url(url)?;
        let url_hash = hash_canonical_url(&canonical_url);

        // 1. Title
        let title = if let Some(title_sel_str) = &recipe.selectors.title {
            let selector = Selector::parse(title_sel_str)
                .map_err(|e| ScraperError::Parse(format!("Invalid title selector: {:?}", e)))?;
            document
                .select(&selector)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        // Fallback title jika kosong
        let title = if title.is_empty() {
            extract_title_fallback(document).unwrap_or_else(|| "Untitled".to_string())
        } else {
            clean_whitespace(&title)
        };

        // 2. Author / Byline
        let byline = if let Some(author_sel_str) = &recipe.selectors.author {
            let selector = Selector::parse(author_sel_str)
                .map_err(|e| ScraperError::Parse(format!("Invalid author selector: {:?}", e)))?;
            document.select(&selector).next().map(|el| {
                clean_whitespace(&el.text().collect::<Vec<_>>().join(" "))
            })
        } else {
            None
        };

        // 3. Top Image
        let top_image = if let Some(img_sel_str) = &recipe.selectors.top_image {
            let selector = Selector::parse(img_sel_str)
                .map_err(|e| ScraperError::Parse(format!("Invalid image selector: {:?}", e)))?;
            document.select(&selector).next().and_then(|el| {
                el.value()
                    .attr("src")
                    .or_else(|| el.value().attr("data-src"))
                    .map(|s| s.to_string())
            })
        } else {
            extract_og_image(document)
        };

        // 4. Content Text & HTML
        let (content_text, content_html) = if let Some(content_sel_str) = &recipe.selectors.content {
            let selector = Selector::parse(content_sel_str)
                .map_err(|e| ScraperError::Parse(format!("Invalid content selector: {:?}", e)))?;

            if let Some(content_element) = document.select(&selector).next() {
                let inner_html = content_element.inner_html();
                let clean_text = clean_whitespace(&content_element.text().collect::<Vec<_>>().join("\n"));
                (clean_text, Some(inner_html))
            } else {
                extract_readability_content(document)
            }
        } else {
            extract_readability_content(document)
        };

        // 5. Summary / Description
        let summary = if let Some(sum_sel_str) = &recipe.selectors.summary {
            let selector = Selector::parse(sum_sel_str)
                .map_err(|e| ScraperError::Parse(format!("Invalid summary selector: {:?}", e)))?;
            document.select(&selector).next().map(|el| {
                clean_whitespace(&el.text().collect::<Vec<_>>().join(" "))
            })
        } else {
            extract_meta_description(document)
        };

        // 6. Tags
        let mut tags = Vec::new();
        if let Some(tag_sel_str) = &recipe.selectors.tags {
            if let Ok(selector) = Selector::parse(tag_sel_str) {
                for el in document.select(&selector) {
                    let t = clean_whitespace(&el.text().collect::<Vec<_>>().join(" "));
                    if !t.is_empty() && !tags.contains(&t) {
                        tags.push(t);
                    }
                }
            }
        }

        Ok(Article {
            id: Uuid::new_v4().to_string(),
            provider_id: recipe.id.clone(),
            original_url: url.to_string(),
            canonical_url,
            url_hash,
            title,
            byline,
            summary,
            content_text,
            content_html,
            published_at: None, // Disempurnakan saat parsing datetime
            scraped_at: Utc::now(),
            top_image,
            tags,
            language: recipe.country.clone(),
            metadata: HashMap::new(),
        })
    }

    /// Ekstraksi fallback murni heuristik (Readability-like)
    fn extract_heuristic(&self, document: &Html, url: &str) -> Result<Article> {
        let canonical_url = normalize_url(url)?;
        let url_hash = hash_canonical_url(&canonical_url);

        let title = extract_title_fallback(document).unwrap_or_else(|| "Untitled".to_string());
        let summary = extract_meta_description(document);
        let top_image = extract_og_image(document);
        let (content_text, content_html) = extract_readability_content(document);

        Ok(Article {
            id: Uuid::new_v4().to_string(),
            provider_id: "generic".to_string(),
            original_url: url.to_string(),
            canonical_url,
            url_hash,
            title,
            byline: None,
            summary,
            content_text,
            content_html,
            published_at: None,
            scraped_at: Utc::now(),
            top_image,
            tags: Vec::new(),
            language: None,
            metadata: HashMap::new(),
        })
    }
}

impl Extractor for DefaultExtractor {
    fn extract(&self, html: &str, url: &str, recipe: Option<&Recipe>) -> Result<Article> {
        let document = Html::parse_document(html);
        if let Some(r) = recipe {
            self.extract_with_recipe(&document, url, r)
        } else {
            self.extract_heuristic(&document, url)
        }
    }
}

fn extract_title_fallback(document: &Html) -> Option<String> {
    // 1. og:title
    if let Ok(sel) = Selector::parse(r#"meta[property="og:title"], meta[name="og:title"]"#) {
        if let Some(el) = document.select(&sel).next() {
            if let Some(content) = el.value().attr("content") {
                return Some(clean_whitespace(content));
            }
        }
    }
    // 2. h1
    if let Ok(sel) = Selector::parse("h1") {
        if let Some(el) = document.select(&sel).next() {
            let text = clean_whitespace(&el.text().collect::<Vec<_>>().join(" "));
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    // 3. title tag
    if let Ok(sel) = Selector::parse("title") {
        if let Some(el) = document.select(&sel).next() {
            return Some(clean_whitespace(&el.text().collect::<Vec<_>>().join(" ")));
        }
    }
    None
}

fn extract_meta_description(document: &Html) -> Option<String> {
    if let Ok(sel) = Selector::parse(r#"meta[name="description"], meta[property="og:description"]"#) {
        if let Some(el) = document.select(&sel).next() {
            if let Some(content) = el.value().attr("content") {
                return Some(clean_whitespace(content));
            }
        }
    }
    None
}

fn extract_og_image(document: &Html) -> Option<String> {
    if let Ok(sel) = Selector::parse(r#"meta[property="og:image"], meta[name="twitter:image"]"#) {
        if let Some(el) = document.select(&sel).next() {
            if let Some(content) = el.value().attr("content") {
                return Some(content.trim().to_string());
            }
        }
    }
    None
}

/// Heuristik mengekstrak body teks terpanjang dari tag <article> atau paragraf-paragraf <p>
fn extract_readability_content(document: &Html) -> (String, Option<String>) {
    // Coba temukan tag <article> lebih dulu
    if let Ok(article_sel) = Selector::parse("article") {
        if let Some(el) = document.select(&article_sel).next() {
            let text = clean_whitespace(&el.text().collect::<Vec<_>>().join("\n\n"));
            if text.len() > 100 {
                return (text, Some(el.inner_html()));
            }
        }
    }

    // Fallback: kumpulkan semua paragraf <p> yang bermakna
    if let Ok(p_sel) = Selector::parse("p") {
        let mut paragraphs = Vec::new();
        for p in document.select(&p_sel) {
            let text = clean_whitespace(&p.text().collect::<Vec<_>>().join(" "));
            // Abaikan paragraf terlalu pendek (biasanya footer, disclaimer, nav)
            if text.len() > 30 {
                paragraphs.push(text);
            }
        }
        if !paragraphs.is_empty() {
            let full_text = paragraphs.join("\n\n");
            return (full_text, None);
        }
    }

    (String::new(), None)
}

fn clean_whitespace(input: &str) -> String {
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(input.trim(), " ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_heuristic() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Judul Berita Contoh - Media</title>
                <meta property="og:description" content="Ini adalah ringkasan berita penting">
                <meta property="og:image" content="https://example.com/cover.jpg">
            </head>
            <body>
                <article>
                    <h1>Judul Berita Penting Sekali</h1>
                    <p>Paragraf pertama memuat informasi sangat berharga mengenai perkembangan ekonomi terkini.</p>
                    <p>Paragraf kedua menjelaskan dampak inflasi terhadap daya beli masyarakat Indonesia secara mendalam.</p>
                </article>
            </body>
            </html>
        "#;

        let extractor = DefaultExtractor::new();
        let article = extractor
            .extract(html, "https://example.com/news/123?utm_source=fb", None)
            .unwrap();

        assert_eq!(article.title, "Judul Berita Penting Sekali");
        assert_eq!(
            article.summary.as_deref(),
            Some("Ini adalah ringkasan berita penting")
        );
        assert_eq!(
            article.top_image.as_deref(),
            Some("https://example.com/cover.jpg")
        );
        assert!(article.content_text.contains("daya beli masyarakat"));
        assert_eq!(article.canonical_url, "https://example.com/news/123");
    }
}
