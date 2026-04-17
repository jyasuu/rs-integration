# Rust Web API — Per-Request In-Memory Cache

A production-grade example of caching database table data in memory for a Rust
web API, with four hard guarantees built into the design.

---

## Architecture

```
HTTP Request
     │
     ▼
 Axum Handler  (clone of Arc<AppState> — zero-cost per request)
     │
     ▼
 ProductCache  (Arc<ProductCacheInner> — shared across all handlers)
     │
     ├─── Cache HIT  ──► return Arc<T>  (no allocation, no DB)
     │
     └─── Cache MISS ──► DB query ──► store in cache ──► return Arc<T>
```

Dependencies:
- **axum** — async web framework
- **moka** — high-performance, concurrent in-memory cache (TinyLFU eviction)
- **sqlx** — async, compile-time-checked SQL for PostgreSQL
- **tokio** — async runtime

---

## The Four Guarantees

### 1. Out-of-Memory Safety

**Problem:** An unbounded cache will eventually exhaust the heap.

**Solution — three-layer defence:**

| Layer | Mechanism | Where |
|-------|-----------|-------|
| Entry cap | `max_capacity(10_000)` hard limit | `cache.rs` |
| Time-based eviction | `time_to_live(300s)` + `time_to_idle(60s)` | `cache.rs` |
| Byte-level cap (optional) | `.weigher(...)` for exact byte budgeting | `cache.rs` (commented out) |

`moka` uses a **TinyLFU** admission policy: frequently-accessed entries
survive eviction; cold entries are dropped first. The cache never grows
beyond `max_capacity`, regardless of traffic volume.

**Sizing guide:**

```
max_capacity × avg_entry_bytes ≤ target_cache_memory_budget
e.g.  10_000 × 400 B ≈ 4 MB
```

---

### 2. Performance

**Problem:** Naive caches acquire a global mutex on every read, making them
slower than no cache under high concurrency.

**Solution:**

- `moka::future::Cache` is based on a lock-free, segmented hash map.
  Reads and writes from different keys never contend.
- Values are stored as `Arc<T>`. A cache hit clones the `Arc` pointer
  (8 bytes + atomic increment) — never the underlying data.
- `get_with` / `try_get_with` are single-call read-or-populate operations
  that avoid the double-lookup pattern (get → miss → insert) which opens
  a race window.

---

### 3. Concurrency (Thundering-Herd Prevention)

**Problem:** When a cached entry expires, hundreds of concurrent requests
for the same key may simultaneously query the database — a "thundering herd"
that can overload the DB and cause latency spikes.

**Solution:** `try_get_with` coalesces concurrent initialisers:

```rust
cache.try_get_with(key, async { db_query().await }).await
//                 ^^^  only ONE of these runs at a time per key
//                       all other callers wait for this future
```

`moka` tracks in-flight initialisers per key. If 50 tasks call
`try_get_with` for the same key simultaneously, exactly **one** DB query
runs. The other 49 tasks await its result. This is equivalent to a
per-key `OnceCell` or `tokio::sync::OnceCell`, but integrated into the
cache lifecycle.

---

### 4. Command Reorder Prevention

**Problem:** Consider this sequence:

```
Thread A: UPDATE products SET name='new' WHERE id=X   ← DB write
Thread A: cache.insert(X, old_value)                  ← BUG: stale write
Thread B: cache.get(X)                                ← returns old_value ❌
```

An out-of-order cache write after a DB mutation corrupts the cache.

**Solution — cache-aside with post-write invalidation:**

```rust
// 1. Write to DB (source of truth)
sqlx::query("UPDATE products ...").execute(&db).await?;

// 2. Invalidate AFTER the write commits
//    invalidate() is async — it waits for any pending background writes
//    to complete before removing the entry.
cache.invalidate_product(id).await;

// 3. Next reader will miss the cache and fetch the fresh row from DB
```

Rules:
- **Never** call `cache.insert()` on a mutation path — use invalidation only.
- Always invalidate **after** the DB transaction commits (not before).
- `invalidate()` is `async` — `await` it. Skipping the await means the
  next read might race against the in-flight eviction.

---

## Running Locally

```bash
# 1. Start PostgreSQL
docker run -d \
  -e POSTGRES_DB=products_dev \
  -e POSTGRES_PASSWORD=postgres \
  -p 5432:5432 postgres:16

# 2. Export connection string
export DATABASE_URL=postgres://postgres:postgres@localhost/products_dev

# 3. Run (migrations auto-apply at startup)
cargo run

# 4. Test
curl -X POST http://localhost:8080/products \
  -H 'Content-Type: application/json' \
  -d '{"name":"Widget","category":"widgets","price_cents":999,"stock":100}'

curl 'http://localhost:8080/products?category=widgets&page=0'
```

## Running Tests (no DB required)

```bash
cargo test
```

The integration tests in `tests/cache_guarantees.rs` mock the database with
an atomic counter, making them fully self-contained.

---

## Tuning Checklist

| Parameter | Default | When to change |
|-----------|---------|----------------|
| `max_capacity` | 10,000 | Lower if entries are large; raise for small entries |
| `time_to_live` | 300 s | Lower for frequently-updated data; raise for static data |
| `time_to_idle` | 60 s | Lower to evict cold entries sooner |
| `max_connections` (DB pool) | 32 | Match to `max_connections` in postgresql.conf |
| `min_connections` (DB pool) | 4 | Match to your idle-request baseline |

---

## File Structure

```
src/
├── main.rs        — server bootstrap, router wiring
├── cache.rs       — ProductCache with all four guarantees
├── handlers.rs    — Axum route handlers (read + write paths)
├── model.rs       — domain types (Product, CreateProduct, …)
├── db.rs          — PgPool construction
└── error.rs       — unified AppError + IntoResponse

migrations/
└── 20240101000000_create_products.sql

tests/
└── cache_guarantees.rs  — self-contained tests for all four guarantees
```
