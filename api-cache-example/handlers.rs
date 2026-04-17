/// handlers.rs — Axum request handlers
///
/// AppState is cloned into every handler for free (Arc internally).
/// The cache handles all concurrency and reorder concerns.
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    cache::ProductCache,
    error::AppError,
    model::{CreateProduct, Product, UpdateProduct},
    db::DbPool,
};

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub cache: ProductCache,
    pub db: DbPool,
}

// ---------------------------------------------------------------------------
// GET /products/:id
// ---------------------------------------------------------------------------

pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Arc<Product>>, AppError> {
    // Returns Arc<Product> — zero-copy clone from the cache to the response.
    let product = state.cache.get_product(id).await?;
    Ok(Json(product))
}

// ---------------------------------------------------------------------------
// GET /products?category=widgets&page=0
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ListQuery {
    pub category: String,
    pub page: Option<u32>,
}

pub async fn list_products(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Arc<Vec<Product>>>, AppError> {
    let page = q.page.unwrap_or(0);
    let products = state.cache.get_products_by_category(&q.category, page).await?;
    Ok(Json(products))
}

// ---------------------------------------------------------------------------
// POST /products
// ---------------------------------------------------------------------------

pub async fn create_product(
    State(state): State<AppState>,
    Json(body): Json<CreateProduct>,
) -> Result<(StatusCode, Json<Product>), AppError> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    let product = sqlx::query_as::<_, Product>(
        "INSERT INTO products (id, name, category, price_cents, stock, updated_at)
         VALUES (gen_random_uuid(), $1, $2, $3, $4, NOW())
         RETURNING *",
    )
    .bind(&body.name)
    .bind(&body.category)
    .bind(body.price_cents)
    .bind(body.stock)
    .fetch_one(&state.db)
    .await?;

    // Invalidate the category list so the new item is visible on next read.
    // No need to pre-populate the by_id entry — cache-aside will fill it lazily.
    state.cache.invalidate_product(product.id).await;

    Ok((StatusCode::CREATED, Json(product)))
}

// ---------------------------------------------------------------------------
// PATCH /products/:id
// ---------------------------------------------------------------------------

pub async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProduct>,
) -> Result<Json<Product>, AppError> {
    // Optimistic: fetch current row (may come from cache) before patching.
    let current = state.cache.get_product(id).await?;

    let updated = sqlx::query_as::<_, Product>(
        "UPDATE products
            SET name        = COALESCE($2, name),
                price_cents = COALESCE($3, price_cents),
                stock       = COALESCE($4, stock),
                updated_at  = NOW()
          WHERE id = $1
         RETURNING *",
    )
    .bind(id)
    .bind(body.name.as_deref().unwrap_or(&current.name))
    .bind(body.price_cents.unwrap_or(current.price_cents))
    .bind(body.stock.unwrap_or(current.stock))
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound(id))?;

    // REORDER SAFETY: invalidate AFTER the DB write commits.
    // Any concurrent reader that bypassed the cache between the write and this
    // invalidate will get either the old cached value or the new DB value —
    // both are acceptable in a cache-aside pattern.
    state.cache.invalidate_product(id).await;

    Ok(Json(updated))
}

// ---------------------------------------------------------------------------
// DELETE /products/:id
// ---------------------------------------------------------------------------

pub async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let rows = sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound(id));
    }

    state.cache.invalidate_product(id).await;
    Ok(StatusCode::NO_CONTENT)
}
