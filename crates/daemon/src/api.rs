use crate::engine::EngineRuntime;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use eay_scraper_core::pipeline::StorageSink;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct ListArticlesQuery {
    pub provider: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ScrapeUrlRequest {
    pub url: String,
    pub recipe_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub total_articles: usize,
    pub loaded_recipes: usize,
}

pub fn create_router(runtime: Arc<EngineRuntime>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/articles", get(list_articles_handler))
        .route("/api/v1/articles/:id", get(get_article_handler))
        .route("/api/v1/recipes", get(list_recipes_handler))
        .route("/api/v1/recipes/reload", post(reload_recipes_handler))
        .route("/api/v1/scrape/url", post(scrape_url_handler))
        .route("/api/v1/scrape/provider/:id", post(scrape_provider_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(runtime)
}

async fn health_handler(State(runtime): State<Arc<EngineRuntime>>) -> impl IntoResponse {
    let total_articles = runtime.storage().count_articles(None).await.unwrap_or(0);
    let loaded_recipes = runtime.recipes().list_recipes().await.len();

    Json(HealthResponse {
        status: "ok",
        version: "0.1.0",
        total_articles,
        loaded_recipes,
    })
}

async fn list_articles_handler(
    State(runtime): State<Arc<EngineRuntime>>,
    Query(params): Query<ListArticlesQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    match runtime
        .storage()
        .get_articles(params.provider.as_deref(), limit, offset)
        .await
    {
        Ok(articles) => (StatusCode::OK, Json(serde_json::to_value(articles).unwrap())).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_article_handler(
    State(runtime): State<Arc<EngineRuntime>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match runtime.storage().get_article_by_id(&id).await {
        Ok(Some(article)) => (StatusCode::OK, Json(serde_json::to_value(article).unwrap())).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Article not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn list_recipes_handler(State(runtime): State<Arc<EngineRuntime>>) -> impl IntoResponse {
    let recipes = runtime.recipes().list_recipes().await;
    Json(recipes)
}

async fn reload_recipes_handler(State(runtime): State<Arc<EngineRuntime>>) -> impl IntoResponse {
    match runtime.recipes().load_all().await {
        Ok(count) => Json(serde_json::json!({ "status": "reloaded", "count": count })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn scrape_url_handler(
    State(runtime): State<Arc<EngineRuntime>>,
    Json(payload): Json<ScrapeUrlRequest>,
) -> impl IntoResponse {
    match runtime.scrape_single_url(&payload.url, payload.recipe_id.as_deref()).await {
        Ok(article) => (StatusCode::OK, Json(serde_json::to_value(article).unwrap())).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn scrape_provider_handler(
    State(runtime): State<Arc<EngineRuntime>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Some(recipe) = runtime.recipes().get_recipe(&id).await {
        let provider_id = id.clone();
        tokio::spawn(async move {
            if let Err(e) = runtime.run_provider(&recipe).await {
                tracing::error!("Error running provider trigger {}: {}", provider_id, e);
            }
        });

        (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "status": "triggered", "provider": id })),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Recipe not found" })),
        )
            .into_response()
    }
}

pub async fn run_server(runtime: Arc<EngineRuntime>, host: &str, port: u16) -> anyhow::Result<()> {
    let app = create_router(runtime);
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("HTTP Control Plane listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
