use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScraperError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network request failed: {0}")]
    Network(String),

    #[error("HTTP status error ({code}): {message}")]
    HttpStatus { code: u16, message: String },

    #[error("Parsing error: {0}")]
    Parse(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Recipe error: {0}")]
    Recipe(String),

    #[error("Extraction failed for URL {url}: {reason}")]
    ExtractionFailed { url: String, reason: String },

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Rate limited on domain {domain}, retry after {retry_after_secs}s")]
    RateLimited { domain: String, retry_after_secs: u64 },

    #[error("Proxy error: {0}")]
    Proxy(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ScraperError>;
