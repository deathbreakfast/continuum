# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| `main` (v0.1.x early release) | Yes |
| Older commits | No |

## Reporting a vulnerability

**Do not open a public GitHub issue** for undisclosed security problems.

Report privately via [GitHub Security Advisories](https://github.com/deathbreakfast/continuum/security/advisories/new) for this repository.

Include:

- Description of the issue and potential impact
- Steps to reproduce (proof-of-concept if available)
- Affected crates, backends, or code paths

## Scope

In scope:

- The Continuum transport log port (`continuum-core`)
- Backend implementations in this repository (`mem`, Surreal, PostgreSQL, SQLite)
- The `continuum-bench` harness when run against local/temporary stores

Out of scope:

- Encryption, key management, and host application wiring (callers own payloads above the port)
- Deployments and infrastructure you configure outside this repo
- Third-party database engines (SurrealDB, PostgreSQL, SQLite) except where Continuum's adapter code is at fault

## Response

This is an early-release project with no formal SLA. Reports will be acknowledged in a reasonable timeframe. Fixes may land on `main` and be documented in advisory release notes when applicable.
