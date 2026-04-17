/// main.rs — Application entry point
///
/// Wires together: DB pool → cache → Axum router → server
mod cache;
mod db;
mod error;
mod handlers;
mod model;

use std::sync::Arc;

use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cache::{CacheConfig, ProductCacheInner};
use handlers::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Logging -----------------------------------------------------------
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,api_cache_example=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // --- Database ----------------------------------------------------------
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/products_dev".into());

    let db = db::connect(&database_url).await?;
    tracing::info!("connected to database");

    // --- Cache -------------------------------------------------------------
    // CacheConfig::default() is a safe starting point.
    // Tune max_capacity and TTLs based on your load profile.
    let cache = Arc::new(ProductCacheInner::new(db.clone(), CacheConfig::default()));

    // --- Router ------------------------------------------------------------
    let state = AppState { cache, db };

    let app = Router::new()
        .route("/products",         get(handlers::list_products))
        .route("/products",         post(handlers::create_product))
        .route("/products/:id",     get(handlers::get_product))
        .route("/products/:id",     patch(handlers::update_product))
        .route("/products/:id",     delete(handlers::delete_product))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // --- Server ------------------------------------------------------------
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
