/// db.rs — Database pool initialisation
use sqlx::PgPool;

pub type DbPool = PgPool;

pub async fn connect(database_url: &str) -> Result<DbPool, sqlx::Error> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        // PERFORMANCE: keep connections warm; avoid cold-start latency per request.
        .min_connections(4)
        // CONCURRENCY: cap total DB connections to prevent pool exhaustion.
        .max_connections(32)
        // Fail fast on bad DB rather than hanging indefinitely.
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;

    // Run migrations at startup so the schema is always current.
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
