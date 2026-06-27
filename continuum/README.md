# continuum (crate)

Overview, quickstart, and feature guide: [../README.md](../README.md).

Public facade re-exporting [`continuum-core`](../continuum-core) and feature-gated backend crates.

## Cargo features

No features are enabled by default. Enable backends explicitly in your dependency:

```toml
continuum = { git = "https://github.com/unified-field-dev/continuum", features = ["mem"] }
# or: features = ["surreal-local"]
# or: default-features = false  # port + DTOs + router only
```

| Feature | Backend crate | Contents |
|---------|---------------|----------|
| `mem` | `continuum-backend-mem` | [`InMemoryLogBackend`](../continuum-backend-mem/src/memory.rs) |
| `surreal-local` | `continuum-backend-surreal` | [`SurrealLocalLogBackend`](../continuum-backend-surreal/src/surreal_local/mod.rs) |
| `postgres` / `sqlite` | `continuum-backend-*` | PostgreSQL and SQLite transport log |
| `telemetry-console` | `continuum-telemetry` | `TelemetrySink`, `InstrumentedLogBackend`, console sink |

## API documentation

**Source of truth:** `cargo doc -p continuum --open`

Architecture, goals, port contracts, and examples live in rustdoc on the public API items and module pages.

**Boundary:** no product constants in this crate — logical destination names and database handles are owned by host wiring.

## Backend wiring

### Surreal-local

`continuum` does **not** own the Surreal client. The host injects an embedded or remote handle:

```rust
use std::sync::Arc;
use continuum::SurrealLocalLogBackend;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

let db: Arc<Surreal<Any>> = /* embedded or remote */;
let backend = SurrealLocalLogBackend::new(db).await?;
// Register on LogRouter at application boot.
```

Use [`SurrealLocalLogBackend::new`](../continuum-backend-surreal/src/surreal_local/mod.rs) for dynamic clients and [`new_embedded_local`](../continuum-backend-surreal/src/surreal_local/mod.rs) for embedded RocksDB — see rustdoc for details.

### PostgreSQL

The backend opens its own pool from a connection URL:

```rust
use continuum::PostgresLogBackend;

let backend = PostgresLogBackend::connect("postgres://user:pass@localhost/continuum").await?;
```

### SQLite

The backend opens (or creates) a local database file:

```rust
use continuum::SqliteLogBackend;

let backend = SqliteLogBackend::new("/var/lib/continuum/transport.db").await?;
```
