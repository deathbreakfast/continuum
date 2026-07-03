# continuum (crate)

Overview, quickstart, and feature guide: [../README.md](../README.md).

Public facade re-exporting [`continuum-core`](../continuum-core) and feature-gated backend crates.

## Cargo features

No features are enabled by default. Enable backends explicitly in your dependency:

```toml
continuum = { git = "https://github.com/unified-field-dev/continuum", features = ["mem"] }
# or: features = ["surreal-local"]
# or: features = ["scylla"]
# or: features = ["tikv-raw"]
# or: default-features = false  # port + DTOs + router only
```

| Feature | Backend crate | Contents |
|---------|---------------|----------|
| `mem` | `continuum-backend-mem` | [`InMemoryLogBackend`](../continuum-backend-mem/src/memory.rs) |
| `surreal-local` | `continuum-backend-surreal` | [`SurrealLocalLogBackend`](../continuum-backend-surreal/src/surreal_local/mod.rs) |
| `postgres` / `sqlite` | `continuum-backend-*` | PostgreSQL and SQLite transport log |
| `scylla` | `continuum-backend-scylla` | [`ScyllaLogBackend`](../continuum-backend-scylla/src/lib.rs), [`ScyllaLogConfig`](../continuum-backend-scylla/src/lib.rs) |
| `tikv-raw` | `continuum-backend-tikv-raw` | [`TikvRawLogBackend`](../continuum-backend-tikv-raw/src/lib.rs), [`TikvRawLogConfig`](../continuum-backend-tikv-raw/src/lib.rs) |
| `telemetry-console` | `continuum-telemetry` | `TelemetrySink`, `InstrumentedLogBackend`, console sink |

## API documentation

**Source of truth:** `cargo doc -p continuum --open`

Architecture, goals, port contracts, and examples live in rustdoc on the public API items and module pages.

**Boundary:** no product constants in this crate — logical destination names and database handles are owned by host wiring.

**Runnable examples** (from the workspace root):

```bash
cargo run -p continuum --example quickstart --features mem
cargo run -p continuum --example router --features mem
cargo run -p continuum --example checkpoint_truncate --features mem
cargo run -p continuum-backend-surreal --example surreal_embedded
```

## Configuration

There is no config file or global settings loader. Integrators wire backends in code; optional env vars apply only to telemetry and the benchmark tool.

### Precedence (library)

1. **Cargo features** — choose which backends and telemetry are linked (`default = []`).
2. **Constructor arguments** — connection URLs, paths, or config structs (`ScyllaLogConfig`, `TikvRawLogConfig`, `SurrealLogConfig`).
3. **Struct `Default`** — fields you omit use documented defaults (e.g. Scylla contact point `127.0.0.1:9042`).
4. **`CONTINUUM_TELEMETRY`** (feature `telemetry-console`) — when using `telemetry_from_env()`: unset defaults to **console**; set to `off`, `0`, `false`, or `none` for a no-op sink.

### Precedence (benchmark tool)

`continuum-bench` is an internal performance tool, not part of the library API:

1. **CLI flags** (`--storage`, `--hardware`, `--tikv-topology`, …) on `run` / `matrix`.
2. **`CONTINUUM_BENCH_*` env vars** — remote endpoints, topology report dimensions, partition/client counts. Full table: [`EXPERIMENTS.md`](../continuum-bench/EXPERIMENTS.md).
3. **Hardcoded defaults** — e.g. `mem` storage, `dev-wsl` hardware, topology fallbacks when env is unset.
4. **Scylla tuning env** (`CONTINUUM_SCYLLA_*`) — merged into `ScyllaLogConfig` over `Default` when the bench opens a Scylla backend.

Remote matrix rows are **skipped** (not failed) when required env vars are missing.

### Library env vars

| Variable | Effect |
|----------|--------|
| `CONTINUUM_TELEMETRY` | `telemetry_from_env()`: unset → console; `off` / `0` / `false` / `none` → no-op |
| `CONTINUUM_APPEND_DEBUG_OPS` | When `1` / `true`, Scylla and TiKV-raw backends record append round-trip counters |

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

Use [`SurrealLocalLogBackend::new`](../continuum-backend-surreal/src/surreal_local/mod.rs) for dynamic clients and [`new_embedded_local`](../continuum-backend-surreal/src/surreal_local/mod.rs) for embedded RocksDB — see rustdoc for details. In-memory Surreal (`mem://`) is shown in the `surreal_embedded` example.

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

### Scylla

Connect with [`ScyllaLogConfig`](../continuum-backend-scylla/src/lib.rs) (contact points, keyspace, idempotency, consistency):

```rust
use continuum::{ScyllaLogBackend, ScyllaLogConfig};

let backend = ScyllaLogBackend::connect(ScyllaLogConfig {
    contact_points: vec!["127.0.0.1:9042".into()],
    keyspace: "continuum".into(),
    ..Default::default()
})
.await?;
```

Local stack: [`infra/scylla`](../infra/scylla). Field-level options are documented on `ScyllaLogConfig` in rustdoc.

### TiKV-raw

Connect via Placement Driver (PD) endpoints:

```rust
use continuum::{TikvRawLogBackend, TikvRawLogConfig};

let backend = TikvRawLogBackend::connect(TikvRawLogConfig {
    pd_endpoints: vec!["127.0.0.1:2379".into()],
})
.await?;
```

Empty `pd_endpoints` defaults to `127.0.0.1:2379`. Local stack: [`infra/tikv-raw`](../infra/tikv-raw).

### Telemetry console

Enable the `telemetry-console` feature and wrap any backend:

```rust
use std::sync::Arc;
use continuum::{ConsoleTelemetry, InMemoryLogBackend, InstrumentedLogBackend};

let backend = InstrumentedLogBackend::new(InMemoryLogBackend::new(), ConsoleTelemetry);
// Or: InstrumentedLogBackend::new(inner, telemetry_from_env())
let _ = Arc::new(backend);
```
