pub mod error;
pub mod extractor;
pub mod model;
pub mod pipeline;
pub mod recipe;
pub mod url_norm;

pub use error::ScraperError;
pub use model::{Article, FeedItem, IngestionBatch, ProviderMetadata};
pub use recipe::Recipe;
