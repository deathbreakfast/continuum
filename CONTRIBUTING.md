# Contributing to Continuum

Thank you for your interest in contributing. This project is an early release (v0.1.x); small, focused changes are easiest to review.

## Prerequisites

- **Nightly Rust** — the workspace pins nightly via [`rust-toolchain.toml`](rust-toolchain.toml). Stable is not supported yet.
- **SQLite dev headers** — required for sqlite backend tests (same as CI):

  ```bash
  sudo apt-get install libsqlite3-dev   # Debian/Ubuntu
  ```

## Verify locally

Run these before opening a pull request:

```bash
cargo test --workspace
cargo check -p continuum --no-default-features
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo test --doc -p continuum-core
cargo test --doc -p continuum-backend-mem
cargo test --doc -p continuum-telemetry
```

Optional: `cargo outdated --root-deps-only --workspace`

## Postgres contract tests

Postgres integration tests are `#[ignore]` by default. With a running Postgres instance:

```bash
CONTINUUM_TEST_POSTGRES_URL=postgres://postgres:postgres@localhost:5432/postgres \
  cargo test -p continuum-backend-postgres -- --ignored
```

CI runs the same against a service container (see [`.github/workflows/ci.yml`](.github/workflows/ci.yml)).

## Pull request expectations

- Keep diffs focused; one logical change per PR when possible.
- Run the verify commands above and ensure CI would pass.
- Update docs, rustdoc, or [`continuum-bench/EXPERIMENTS.md`](continuum-bench/EXPERIMENTS.md) when behavior or benchmarks change.
- Do not commit secrets, `.env` files, or compiled binaries.

## Benchmarks

Synthetic benchmarks live in `continuum-bench`. See [`continuum-bench/EXPERIMENTS.md`](continuum-bench/EXPERIMENTS.md) for experiment IDs and run commands.

Reports are written to `profiling/continuum-bench/reports/`. Commit JSON reports only when intentionally updating published baselines (and note the change in the PR).

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). By participating, you agree to uphold it.

## Security

Do not open public issues for security vulnerabilities. See [SECURITY.md](SECURITY.md).
