# Database (Phase 1)

PostgreSQL via `sqlx`, per `HARBORY_README.md` §4. Schema lives in
[`crates/control-plane/migrations/0001_init.sql`](../crates/control-plane/migrations/0001_init.sql),
applied automatically on startup via `sqlx::migrate!` (`Store::connect`).

## Tables

| Table | Purpose |
|---|---|
| `accounts` | Minimal stand-in (`id`, `email`, `created_at`) — just enough to own agents and pairing tokens. Real auth is Phase 5; don't extend this table until then. |
| `agents` | One row per paired agent: `id`, `account_id`, `public_key`, `public_key_fingerprint`, `status` (`active`/`revoked`), timestamps. |
| `pairing_tokens` | `token_hash` (SHA-256 of plaintext, never the plaintext itself) as primary key, owning `account_id`, `expires_at`, `consumed_at`/`consumed_by_agent_id` (both null until used). |
| `audit_log` | Append-only security events: `event_type`, optional `account_id`/`agent_id`, `detail` (JSONB), `created_at`. Populated now for pairing success/reuse/expiry and credential fingerprint mismatches; consumed by Phase 6 alerting later. |

## Deviation from the roadmap: runtime-checked queries, not `query!`

`HARBORY_README.md` §4 picks sqlx specifically for "compile-time checked
queries" (the `sqlx::query!`/`query_as!` macros). Phase 1 uses the
runtime-checked forms (`sqlx::query`, `sqlx::query_as::<_, T>`) instead.

**Why:** the compile-time macros need either a live database reachable at
`cargo build` time via `DATABASE_URL`, or a committed `.sqlx` offline query
cache (generated with `cargo sqlx prepare`). Neither exists yet — there's no
CI pipeline to own that cache, and requiring a live DB just to `cargo
build` would break for anyone without Postgres running locally.

**How to apply:** once CI is set up, switch to the macro forms and commit
the `.sqlx` cache — flagged here so it isn't forgotten, not because the
current approach is wrong long-term.

## Local dev database

Phase 1 development and the integration tests in
`crates/control-plane/tests/registration.rs` run against a dedicated
Postgres container (not any other project's database):

```bash
docker run -d --name harbory-postgres \
  -e POSTGRES_USER=harbory -e POSTGRES_PASSWORD=harbory_dev_password -e POSTGRES_DB=harbory \
  -p 55433:5432 postgres:16-alpine
```

Default `DATABASE_URL` baked into `Store::connect` callers (overridable via
env): `postgres://harbory:harbory_dev_password@localhost:55433/harbory`.
