/// cache.rs — Per-request in-memory cache layer
///
/// Design guarantees:
///
/// 1. OUT-OF-MEMORY SAFETY
///    - `moka` enforces a hard max_capacity (entry count) and optional weigher
///      (byte-based size). When the cache is full it evicts via TinyLFU policy —
///      high-frequency entries survive; rarely-used ones are dropped before the
///      process ever approaches OOM.
///    - TTL (time-to-live) + TTI (time-to-idle) bound total memory across time:
///      stale entries are automatically removed even if capacity is not exceeded.
///
/// 2. PERFORMANCE
///    - `moka::future::Cache` is a fully async, lock-free concurrent map based on
///      a segmented ConcurrentHashMap. Reads never block writers.
///    - `.get_with()` / `.try_get_with()` are the key APIs: they coalesce multiple
///      concurrent requests for the *same key* into a single database fetch
///      (thundering-herd protection built-in).
///
/// 3. CONCURRENCY
///    - The cache is `Clone + Send + Sync` and safe to share across Tokio tasks
///      and Axum handlers with zero additional locking from your side.
///
/// 4. COMMAND-REORDER PREVENTION
///    - `.try_get_with()` guarantees that, for a given key, only ONE initialiser
///      future runs at a time. Late callers wait for the in-flight future and
///      receive its result — they never start a second DB query that could arrive
///      out-of-order and overwrite a fresher value.
///    - Write-through invalidation uses `.invalidate()` (async, waits for
///      pending writes to finish) so a stale read cannot be served after a
///      committed mutation.
use std::{sync::Arc, time::Duration};

use moka::future::Cache;
use uuid::Uuid;

use crate::{
    db::DbPool,
    error::AppError,
    model::Product,
};

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

/// A cheaply-clonable handle to the shared product cache.
pub type ProductCache = Arc<ProductCacheInner>;

// ---------------------------------------------------------------------------
// Cache configuration
// ---------------------------------------------------------------------------

/// Tunables — change these or load from config/env in production.
pub struct CacheConfig {
    /// Hard cap on number of cached entries.
    /// Each entry is ~200–400 bytes depending on Product size.
    /// 10_000 entries × 400 B ≈ 4 MB maximum — well within typical limits.
    pub max_capacity: u64,

    /// Entries expire this long after they were *written* into the cache.
    pub time_to_live: Duration,

    /// Entries expire this long after the *last read* — catches cold entries
    /// sooner than TTL alone.
    pub time_to_idle: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            time_to_live: Duration::from_secs(300),  // 5 min
            time_to_idle: Duration::from_secs(60),   // 1 min idle
        }
    }
}

// ---------------------------------------------------------------------------
// Inner cache wrapper
// ---------------------------------------------------------------------------

pub struct ProductCacheInner {
    /// Single-product cache keyed by UUID.
    by_id: Cache<Uuid, Arc<Product>>,

    /// List cache keyed by (category, page) — demonstrates composite keys.
    /// Arc<Vec<..>> avoids cloning the whole vector on every cache hit.
    by_category: Cache<(String, u32), Arc<Vec<Product>>>,

    /// A reference to the DB pool so the cache can self-populate (cache-aside).
    db: DbPool,
}

impl ProductCacheInner {
    pub fn new(db: DbPool, cfg: CacheConfig) -> Self {
        let by_id = Cache::builder()
            .max_capacity(cfg.max_capacity)
            .time_to_live(cfg.time_to_live)
            .time_to_idle(cfg.time_to_idle)
            // Optional: byte-level weigher for tighter OOM control.
            // The weight of each entry is passed to the eviction policy.
            // .weigher(|_k, v: &Arc<Product>| {
            //     // Rough size estimate in bytes; tune per your actual struct.
            //     (std::mem::size_of::<Product>() + v.name.len() + 16) as u32
            // })
            .build();

        let by_category = Cache::builder()
            .max_capacity(256) // far fewer list results
            .time_to_live(cfg.time_to_live)
            .time_to_idle(cfg.time_to_idle)
            .build();

        Self { by_id, by_category, db }
    }

    // -----------------------------------------------------------------------
    // Public read API
    // -----------------------------------------------------------------------

    /// Fetch a single product by ID.
    ///
    /// CONCURRENCY + REORDER:
    ///   `try_get_with` ensures that concurrent requests for the same `id`
    ///   share a single DB future.  The initialiser runs exactly once; all
    ///   other callers await its result.  No second query can overwrite the
    ///   first result out-of-order.
    pub async fn get_product(&self, id: Uuid) -> Result<Arc<Product>, AppError> {
        self.by_id
            .try_get_with(id, self.fetch_product_from_db(id))
            .await
            .map_err(|e| AppError::Cache(e.to_string()))
    }

    /// Fetch a page of products in a category.
    pub async fn get_products_by_category(
        &self,
        category: &str,
        page: u32,
    ) -> Result<Arc<Vec<Product>>, AppError> {
        let key = (category.to_owned(), page);

        self.by_category
            .try_get_with(key, self.fetch_category_page_from_db(category, page))
            .await
            .map_err(|e| AppError::Cache(e.to_string()))
    }

    // -----------------------------------------------------------------------
    // Write-through invalidation
    // -----------------------------------------------------------------------

    /// Call this after a successful INSERT/UPDATE/DELETE for a product.
    ///
    /// REORDER SAFETY: `.invalidate()` is asynchronous — it waits until
    /// any pending background write for this key completes before removing
    /// the entry. Subsequent reads will hit the DB and re-populate.
    pub async fn invalidate_product(&self, id: Uuid) {
        self.by_id.invalidate(&id).await;
        // Also invalidate all category pages — they may contain the product.
        self.by_category.invalidate_all();
    }

    /// Drain the entire cache (e.g., after a bulk import).
    pub async fn invalidate_all(&self) {
        self.by_id.invalidate_all();
        self.by_category.invalidate_all();
        // run_pending_tasks() flushes eviction events synchronously.
        self.by_id.run_pending_tasks().await;
        self.by_category.run_pending_tasks().await;
    }

    // -----------------------------------------------------------------------
    // Internal DB fetchers
    // -----------------------------------------------------------------------

    async fn fetch_product_from_db(&self, id: Uuid) -> Result<Arc<Product>, AppError> {
        let product = sqlx::query_as::<_, Product>(
            "SELECT id, name, category, price_cents, stock, updated_at
               FROM products
              WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?
        .ok_or(AppError::NotFound(id))?;

        tracing::debug!(%id, "cache miss — fetched product from DB");
        Ok(Arc::new(product))
    }

    async fn fetch_category_page_from_db(
        &self,
        category: &str,
        page: u32,
    ) -> Result<Arc<Vec<Product>>, AppError> {
        const PAGE_SIZE: i64 = 20;
        let offset = i64::from(page) * PAGE_SIZE;

        let products = sqlx::query_as::<_, Product>(
            "SELECT id, name, category, price_cents, stock, updated_at
               FROM products
              WHERE category = $1
              ORDER BY name
              LIMIT $2 OFFSET $3",
        )
        .bind(category)
        .bind(PAGE_SIZE)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        tracing::debug!(category, page, count = products.len(), "cache miss — fetched page from DB");
        Ok(Arc::new(products))
    }
}
