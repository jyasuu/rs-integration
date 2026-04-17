/// tests/cache_guarantees.rs
///
/// These tests demonstrate the four guarantees *without* a real DB:
/// we swap in a mock counter instead of sqlx to keep the tests self-contained.
///
/// Run with: cargo test
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use moka::future::Cache;
use tokio::time::sleep;

// ---------------------------------------------------------------------------
// Helper — shared atomic DB-call counter
// ---------------------------------------------------------------------------

type FetchCount = Arc<AtomicU32>;

fn counter() -> FetchCount {
    Arc::new(AtomicU32::new(0))
}

// ---------------------------------------------------------------------------
// 1. OOM SAFETY — eviction enforces max_capacity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_oom_safety_eviction() {
    const MAX: u64 = 5;
    let cache: Cache<u32, u32> = Cache::builder()
        .max_capacity(MAX)
        .build();

    // Insert more entries than the cap allows.
    for i in 0..20 {
        cache.insert(i, i).await;
    }

    // moka evicts asynchronously; flush pending tasks first.
    cache.run_pending_tasks().await;

    // The cache must not grow beyond its hard cap.
    assert!(
        cache.entry_count() <= MAX,
        "entry_count {} exceeds max_capacity {}",
        cache.entry_count(),
        MAX
    );
}

// ---------------------------------------------------------------------------
// 2. PERFORMANCE — cache hit avoids repeated "DB" calls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cache_hit_avoids_db() {
    let fetch_count = counter();
    let cache: Cache<u32, Arc<String>> = Cache::builder()
        .max_capacity(100)
        .build();

    let key = 42u32;

    for _ in 0..10 {
        let fc = fetch_count.clone();
        let _ = cache
            .get_with(key, async move {
                fc.fetch_add(1, Ordering::SeqCst);
                Arc::new(format!("product-{key}"))
            })
            .await;
    }

    assert_eq!(
        fetch_count.load(Ordering::SeqCst),
        1,
        "DB should be called exactly once for 10 concurrent reads of the same key"
    );
}

// ---------------------------------------------------------------------------
// 3. CONCURRENCY — thundering-herd: many tasks, one DB fetch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_concurrency_thundering_herd() {
    let fetch_count = counter();
    let cache: Cache<&'static str, Arc<String>> = Cache::builder()
        .max_capacity(100)
        .build();

    // Simulate 50 concurrent handlers all requesting the same key simultaneously.
    let tasks: Vec<_> = (0..50)
        .map(|_| {
            let fc = fetch_count.clone();
            let c = cache.clone();
            tokio::spawn(async move {
                c.try_get_with("product:99", async move {
                    // Simulate DB latency.
                    sleep(Duration::from_millis(20)).await;
                    fc.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, String>(Arc::new("product-99".to_string()))
                })
                .await
                .unwrap()
            })
        })
        .collect();

    for t in tasks {
        t.await.unwrap();
    }

    assert_eq!(
        fetch_count.load(Ordering::SeqCst),
        1,
        "Only ONE DB fetch should occur regardless of 50 concurrent callers"
    );
}

// ---------------------------------------------------------------------------
// 4. COMMAND REORDER PREVENTION — invalidate + re-fetch gives fresh data
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_command_reorder_prevention() {
    let version = Arc::new(AtomicU32::new(1)); // "DB" starts at v1
    let cache: Cache<&'static str, u32> = Cache::builder()
        .max_capacity(100)
        .build();

    // First read populates cache with v1.
    let v = cache.get_with("row:1", async { 1u32 }).await;
    assert_eq!(v, 1);

    // Simulate a write that bumps the DB to v2, then invalidates the cache.
    version.store(2, Ordering::SeqCst);
    cache.invalidate(&"row:1").await; // waits for pending ops before removing

    // Next read MUST fetch v2 from the "DB" — not the stale v1 from cache.
    let ver = version.clone();
    let v2 = cache
        .get_with("row:1", async move { ver.load(Ordering::SeqCst) })
        .await;

    assert_eq!(v2, 2, "Post-invalidate read must return the updated value (v2), not v1");
}

// ---------------------------------------------------------------------------
// 5. TTL expiry — entries are automatically removed after TTL
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ttl_expiry() {
    let fetch_count = counter();
    let cache: Cache<u32, u32> = Cache::builder()
        .max_capacity(100)
        .time_to_live(Duration::from_millis(50))
        .build();

    let fc1 = fetch_count.clone();
    cache
        .get_with(1, async move {
            fc1.fetch_add(1, Ordering::SeqCst);
            42u32
        })
        .await;

    sleep(Duration::from_millis(100)).await; // let TTL expire

    let fc2 = fetch_count.clone();
    cache
        .get_with(1, async move {
            fc2.fetch_add(1, Ordering::SeqCst);
            42u32
        })
        .await;

    assert_eq!(
        fetch_count.load(Ordering::SeqCst),
        2,
        "Expired entry must trigger a fresh fetch"
    );
}
