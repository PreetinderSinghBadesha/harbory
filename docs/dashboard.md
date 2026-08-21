# Dashboard (Phase 5)

## Stack decisions (both explicitly deferred until this phase)

**Frontend: React + Vite + TypeScript**, in `frontend/`. Talks to the
existing JSON API (`crates/control-plane/src/http.rs`) rather than the
control plane rendering HTML — chosen over a server-rendered
Rust+htmx approach specifically so the UI can stay a normal, familiar
React app; the trade-off is a second toolchain (npm/node) alongside the
Rust one that didn't exist before this phase.

**Auth + accounts: Supabase**, both database and auth. This is a real
architectural pivot, not an incremental choice — flagging it explicitly
per §0's "don't deviate without discussion" rule, even though it was the
user's own decision, not something decided unilaterally. Concretely:

- Supabase Cloud (hosted), not self-hosted — the project's Postgres
  instance and its Auth service (GoTrue) both live there.
- Auth is delegated entirely to Supabase: email/password *and* OAuth
  (e.g. GitHub) are handled by `@supabase/supabase-js` on the frontend.
  The control plane never sees a password — it only ever verifies a
  Supabase-issued JWT.
- "Migrate everything to Supabase's Postgres" (the user's explicit choice
  over keeping two databases): Supabase's Postgres is vanilla Postgres
  underneath, so the existing sqlx migrations (0001-0004) apply there
  unchanged. `DATABASE_URL` for a real deployment points at Supabase's
  connection string instead of the local `harbory-postgres` container.

## Required setup (you, not Claude Code — account creation is out of scope for the agent)

1. Create a project at [supabase.com](https://supabase.com/dashboard) (or use an existing one).
2. **Project Settings -> API**: Project URL, `anon` public key.
3. **Project Settings -> API -> JWT Settings**: JWT Secret.
4. **Project Settings -> Database**: the Postgres connection string.
5. **Authentication -> Providers**: email is on by default; enable GitHub
   (or whichever OAuth provider(s) you want) here.
6. Backend: set `DATABASE_URL` (step 4) and `SUPABASE_JWT_SECRET` (step 3)
   as env vars for `harbory-control-plane` — see `crates/control-plane/src/main.rs`.
   `SUPABASE_JWT_SECRET` has no fallback; the process refuses to start
   without it (see "Fail fast" note below).
7. Frontend: copy `frontend/.env.example` to `frontend/.env` and fill in
   `VITE_SUPABASE_URL` / `VITE_SUPABASE_ANON_KEY` (steps 2) and
   `VITE_API_URL` (the control plane's HTTP address).

## How a request gets authenticated

1. Frontend: user signs in via `@supabase/supabase-js` (email/password or
   OAuth). Supabase issues a session with a JWT.
2. `apiFetch` (`frontend/src/lib/api.ts`) attaches that JWT as
   `Authorization: Bearer <token>` on every call to the control plane.
3. Backend: `AuthenticatedAccount` (`crates/control-plane/src/auth.rs`),
   an axum extractor, verifies the JWT's HS256 signature against
   `SUPABASE_JWT_SECRET` and checks `aud == "authenticated"` (Supabase's
   convention for user-facing tokens) and `exp`.
4. On success, it calls `Store::get_or_create_account_by_id(sub, email)` —
   idempotent, runs on *every* authenticated request, not just first
   login — which provisions/updates the local `accounts` row for that
   Supabase user id.
5. Every handler that's scoped to a specific `agent_id` also calls
   `require_owned_agent`, which 404s (not 403 — same don't-leak-existence
   rationale as the pairing RPC in `docs/protocol.md`) if the agent
   doesn't exist or belongs to a different account.

**Fail fast:** `SUPABASE_JWT_SECRET` has no insecure default; the control
plane refuses to start without it, rather than running with every endpoint
effectively open. Matches the "fail fast rather than silently degrade"
pattern from Phase 3's Docker connection check.

## Deliberate deviation: no hard FK from `accounts.id` to `auth.users.id`

The textbook Supabase pattern is `create table public.profiles (id uuid
references auth.users on delete cascade, ...)` — a hard foreign key into
Supabase's own `auth` schema. This project does *not* do that.
`accounts.id` is only ever set to a Supabase user's `sub` claim by
application code (`get_or_create_account_by_id`); there's no DB-level
constraint enforcing it.

**Why:** a hard FK means `accounts` can only ever be populated against a
database that actually has Supabase's `auth` schema — i.e. only a real
Supabase project. Every integration test written in Phases 1-4 (pairing,
container/proxy reconciliation, the stream handshake — 60 of the 67 tests
as of this phase) creates test accounts via the plain `create_account`
path and runs against a local, vanilla `postgres:16-alpine` container that
has no `auth` schema at all. Hard-FKing would force *all* of those tests
onto a live Supabase project too — slower, and coupling business-logic
tests that have nothing to do with authentication to an external cloud
dependency. The chosen trade-off: two `Store` methods exist side by side
(`create_account` for tests/dev tooling, `get_or_create_account_by_id` for
the real Supabase-authenticated path — see `crates/control-plane/src/store/accounts.rs`),
and local Postgres stays the fast, dependency-free path for everything
that isn't specifically testing auth (which now has its own suite,
`crates/control-plane/tests/http.rs`, run against local Postgres with a
synthetic JWT rather than a real Supabase project).

## CORS

`CorsLayer::permissive()` on the whole HTTP router — any origin, method,
header. This is deliberately looser than a typical API would allow, and
safe specifically *because* auth is Bearer-token-based, not cookies: a
malicious page on another origin can get the browser to make a request,
but it cannot get the browser to attach a token it doesn't already
possess (unlike cookies, which the browser attaches automatically).
Revisit if this API ever adds cookie-based auth for anything.

## What's built

- `crates/control-plane/src/auth.rs` — JWT verification, the
  `AuthenticatedAccount` extractor, account provisioning.
- Every existing HTTP endpoint now requires auth and, where scoped to an
  agent, ownership (`require_owned_agent` in `http.rs`).
- New endpoints: `GET /me`, `POST /pairing-tokens` (issues a token for the
  authenticated account — the backend for "generate/display a pairing
  token"), `POST /agents/{id}/revoke` (flips `agents.status` to
  `'revoked'` — `verify_agent_credential`, already built in Phase 1,
  already rejects non-`'active'` agents, so this is what actually lets an
  operator trigger that path).
- `frontend/`: `Login` (email/password + GitHub OAuth), `Dashboard`
  (agent list with online/offline/status, revoke button, pairing-token
  generator showing the token once with the install command), `AgentDetail`
  (container and proxy-route deployment forms + desired/observed tables).

## What's not verified live, and why

Every previous phase's smoke test ran the real compiled binaries against
real infrastructure this environment could stand up itself (real
Postgres, real Docker; Phase 4 was the first partial exception, for
nginx). This phase is a full exception: creating a Supabase project is an
account-creation step outside what Claude Code can do on the user's
behalf, so the actual login flow — real signup, real OAuth handoff, a
real Supabase-issued JWT reaching the real backend — has not been
exercised end-to-end as of this write-up.

What *was* verified:
- `crates/control-plane/src/auth.rs`'s JWT verification logic: 4 unit
  tests with synthetic HS256 tokens (valid, wrong secret, expired, wrong
  audience).
- The full HTTP auth gate through the real router (`Router::oneshot`, no
  live Supabase): missing token, garbage token, cross-account access
  denial, ownership success, pairing-token issuance, revoke — 7
  integration tests in `crates/control-plane/tests/http.rs`, against real
  local Postgres.
- The frontend builds (`tsc --noEmit`, `vite build`) and the login page
  renders correctly *without* Supabase configured — shows the setup
  warning rather than crashing, confirmed in a real browser.

Once real Supabase credentials are available: sign up, sign in, OAuth,
confirm a real JWT round-trips through `AuthenticatedAccount` correctly,
and pair/deploy/revoke an agent through the actual dashboard UI rather
than only through `curl`/integration tests.
