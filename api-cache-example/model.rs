/// model.rs — Domain types
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    /// Price stored as integer cents to avoid floating-point issues.
    pub price_cents: i64,
    pub stock: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProduct {
    pub name: String,
    pub category: String,
    pub price_cents: i64,
    pub stock: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProduct {
    pub name: Option<String>,
    pub price_cents: Option<i64>,
    pub stock: Option<i32>,
}
