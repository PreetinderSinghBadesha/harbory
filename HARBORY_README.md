# Harbory
A distributed control plane and agent system, built entirely in Rust, for lightweight infrastructure orchestration. A central web platform coordinates with small, high-performance agents running on remote VMs, which handle container deployment, service management, and reverse proxy configuration — without the operational weight of Kubernetes.
---
## 0. Instructions for Claude Code
**Read this section first, every session.**
- This document is the source of truth for architecture and decisions. Do not deviate from the design here without flagging the change explicitly to the user first.
- **Docs-as-you-go rule:** at the end of every phase (see Roadmap below), before starting the next phase, update this file's **Progress Log** (Section 7) with: what was built, any deviations from plan, any new decisions made, and any open questions. This is not optional — treat it as part of "done."
- If a phase produces new architectural decisions (e.g., exact message schemas, DB schema, error handling conventions), also write/update a dedicated doc under `/docs` (e.g., `/docs/protocol.md`, `/docs/database.md`) rather than letting that knowledge live only in code comments.
- Work in phases, in order. Don't jump ahead to later-phase features (e.g., don't build multi-region HA in Phase 1).
- Prefer boring, well-supported crates over cutting-edge ones. Stability > novelty for a control plane.
- Every new crate dependency added should be briefly justified in a comment in `Cargo.toml` or in the relevant `/docs` file.
- Write tests alongside features, not after. Especially for: token consumption logic, credential verification, and reconciliation logic — these are the security- and correctness-critical paths.
---
## 1. Project Goal
Build a fast, memory-safe, low-overhead orchestration tool that:
- Lets a user manage containers, services, and reverse proxy config across many remote VMs from one dashboard
- Avoids the complexity/resource overhead of Kubernetes
- Uses Rust throughout for performance and safety
- Maintains a real-time, reliable sync between control plane and agents
**Explicitly out of scope for v1:** multi-region control plane HA, arbitrary workload scheduling, plugin/CRD-style extensibility, non-container process management. These may come later — do not build for them prematurely.
---
## 2. Architecture Overview
```
┌─────────────────────┐         gRPC (bi-directional stream, mTLS/token-auth)
│   Control Plane      │◄───────────────────────────────────────┐
│  (central web server)│                                          │
│                       │                                          │
│  - Web dashboard      │                                          │
│  - Auth / accounts    │                                          │
│  - Pairing tokens      │                                          │
│  - Agent registry      │                                          │
│  - Command dispatch     │                                          │
│  - State store (DB)      │                                          │
└─────────────────────┘                                          │
                                                                    │
                                                        ┌───────────▼───────────┐
                                                        │        Agent           │
                                                        │  (runs on remote VM)   │
                                                        │                        │
                                                        │  - Local keypair        │
                                                        │  - Container mgmt        │
                                                        │  - Nginx reconfig          │
                                                        │  - Heartbeat/state report   │
                                                        └────────────────────────┘
```
### Components
| Component | Responsibility |
|---|---|
| **Control Plane (server)** | Web dashboard, account/auth management, pairing token issuance, agent identity registry, command dispatch, state storage, reconciliation logic |
| **Agent** | Runs on each managed VM. Registers via pairing token, holds a long-lived signed credential, maintains a persistent authenticated stream to the control plane, executes commands (container ops, proxy config), reports local state and heartbeats |
| **Shared protocol crate** | Rust crate shared between control plane and agent defining the gRPC service, message types, and versioning — single source of truth for the wire protocol |
---
## 3. Security Model (locked in — do not redesign without discussion)
- **Pairing token**: short-lived (5–10 min), single-use, tied to a specific user account. Used only to bootstrap a new agent. Burned immediately on successful use.
- **Agent identity**: agent generates its own keypair locally on first boot, before any network call. Public key + pairing token sent to control plane on registration.
- **Long-lived credential**: control plane issues a signed credential (cert or signed token) binding `agent_id` ↔ `account_id` ↔ `public_key_fingerprint`. Stored locally on the VM (`0600`, root-owned).
- **Ongoing auth**: every request/stream from the agent is authenticated using this credential, not IP address. IP is logged for anomaly detection only, never used as a trust signal.
- **Misuse detection**:
  - Reuse of an already-consumed pairing token → reject + notify account owner.
  - Credential presented with a public key fingerprint mismatch for a known `agent_id` → reject + stronger "possible compromise" alert.
- **Revocation**: user can revoke an agent from the dashboard. Revoked agents can only rejoin via a brand-new pairing token (manual re-pair).
- **Transient disconnects** (network blips, reboots) do **not** require re-pairing — the agent reconnects automatically using its stored credential.
---
## 4. Tech Stack (initial picks — revisit only with reason)
| Concern | Choice | Why |
|---|---|---|
| Language | Rust (both control plane & agent) | Shared types, memory safety, performance |
| Control plane ↔ agent transport | gRPC, bi-directional stream (`tonic`) | Single persistent authenticated channel for commands, state, heartbeats |
| TLS / credential layer | `rustls` | Fast, safe, well-maintained, pairs naturally with `tonic` |
| Agent keypair generation | `ed25519-dalek` or `ring` | Fast, well-audited |
| Control plane web dashboard backend | `axum` | Ergonomic, integrates well with `tonic`/`tower` ecosystem |
| Database | PostgreSQL (via `sqlx`) | Relational fit for accounts/agents/audit logs; `sqlx` gives compile-time checked queries |
| Container management on agent | Docker/OCI via `bollard` (Docker API client) | v1 scope: Docker only, not a custom runtime |
| Reverse proxy management | Template + validate + reload Nginx config | Validate config before reload; use graceful reload, not restart |
| Frontend | TBD — not the focus of this doc | Decide separately when Phase 4 starts |
---
## 5. Roadmap / Phases
Work through these **in order**. Each phase ends with a doc update (see Section 0).
### Phase 1 — Protocol & Identity Foundations
- [ ] Define shared protocol crate: gRPC service definitions (`.proto`), core message types
- [ ] Implement pairing token generation + single-use consumption logic (control plane)
- [ ] Implement agent keypair generation + pairing handshake (agent side)
- [ ] Implement credential issuance and storage
- [ ] Implement credential verification on stream connect (control plane)
- [ ] Tests: token reuse rejection, credential mismatch rejection, happy-path pairing
- [ ] **Doc update**: `/docs/protocol.md` — exact message schemas, `/docs/security.md` — finalized token/credential lifecycle
### Phase 2 — Persistent Connection & Heartbeats
- [ ] Implement the persistent bi-directional gRPC stream
- [ ] Heartbeat messages over the stream (not a separate endpoint)
- [ ] Control plane: mark agent online/offline based on missed heartbeats
- [ ] Agent: auto-reconnect logic on disconnect, using stored credential (no re-pairing)
- [ ] Dashboard: basic agent list showing online/offline status
- [ ] **Doc update**: `/docs/connection-lifecycle.md` — reconnect/backoff behavior, heartbeat interval and thresholds
### Phase 3 — Command Execution: Containers
- [ ] Define command message types (deploy, stop, remove container, etc.)
- [ ] Agent: execute container commands via `bollard`
- [ ] Agent: report local container state back to control plane
- [ ] Control plane: basic desired-state vs observed-state model (this is the reconciliation foundation — don't skip)
- [ ] **Doc update**: `/docs/reconciliation.md` — how desired/observed state is diffed and converged
### Phase 4 — Reverse Proxy Management
- [ ] Define proxy config command types
- [ ] Agent: template Nginx config, **validate before applying** (`nginx -t` equivalent check), graceful reload
- [ ] Handle race conditions: concurrent config changes should serialize, not clobber
- [ ] **Doc update**: `/docs/proxy-management.md` — validation and rollback strategy
### Phase 5 — Dashboard / UX
- [ ] Account/login system
- [ ] Agent pairing UI (generate/display pairing token + install command)
- [ ] Agent management UI (list, status, revoke)
- [ ] Deployment UI (trigger container/proxy changes)
- [ ] **Doc update**: `/docs/dashboard.md`
### Phase 6 — Hardening & Observability
- [ ] Audit logging for security-relevant events (pairing attempts, credential mismatches, revocations)
- [ ] Alerting/notifications (email or in-dashboard) for misuse signals
- [ ] Metrics/logging for agent and control plane health
- [ ] **Doc update**: `/docs/observability.md`
*(Later phases — multi-region control plane, HA, broader runtime support — are explicitly deferred; revisit only after Phase 6 is stable.)*
---
## 6. Directory Structure (proposed)
```
harbory/
├── crates/
│   ├── protocol/        # shared gRPC/proto definitions + message types
│   ├── control-plane/   # server: dashboard backend, agent registry, dispatch
│   ├── agent/            # binary that runs on remote VMs
│   └── common/            # shared utils (crypto helpers, config, etc.)
├── docs/
│   ├── protocol.md
│   ├── security.md
│   ├── connection-lifecycle.md
│   ├── reconciliation.md
│   ├── proxy-management.md
│   ├── dashboard.md
│   └── observability.md
├── frontend/              # React + Vite + TypeScript dashboard (Phase 5)
├── HARBORY_README.md      # this file
└── Cargo.toml              # workspace root
```
---
## 7. Progress Log
*(Claude Code: append entries here at the end of each phase. Don't overwrite previous entries.)*
### Phase 1 — Complete (2026-08-19)
- Status: `crates/protocol` (proto + generated types), `crates/common` (Ed25519 keypair, fingerprint, signed credential format), `crates/control-plane` (Postgres-backed pairing token + agent store, `PairingService.Register` gRPC impl, audit logging), `crates/agent` (local identity + pairing handshake CLI) all build and pass tests. 11 unit tests in `harbory-common` (crypto/credential roundtrips, tamper/wrong-key rejection). 7 integration tests in `harbory-control-plane` against a real Postgres (happy-path pairing, unknown/reused-token rejection, concurrent-registration race safety, credential fingerprint mismatch, wrong-signer rejection) — all passing. Also smoke-tested the actual `harbory-control-plane` and `harbory-agent` binaries end-to-end over a live gRPC connection (pairing succeeds, reuse is rejected, both are audit-logged) — see terminal transcript in this phase's session.
- Deviations from plan:
  - **Credential mechanism decided as Ed25519-signed token, not mTLS** — resolves the open question below. See `docs/security.md` for full rationale (no CA/cert-rotation infrastructure needed; still satisfies the locked security model's "cert or signed token" wording). Decided with the user in-session, not unilaterally.
  - **sqlx compile-time `query!` macros not used yet** — runtime-checked `query`/`query_as` instead, because the macros need either a live DB at build time or a committed offline query cache, and neither exists without CI. See `docs/database.md`. Revisit once CI exists.
  - **Proof-of-possession (agent signing a connect-time challenge) and transport TLS (rustls) are explicitly deferred to Phase 2**, since both only make sense once the persistent stream exists. `Store::verify_agent_credential` implements and tests everything else (signature, revocation, fingerprint match) now so it's ready to wire in.
- Open questions: see updated §8 below — three of the four original questions are now resolved.

### Phase 2 — Complete (2026-08-20)
- Status: added `AgentStreamService.Stream` (bidi RPC — named `Stream` not `Connect`, since tonic's generated client already has a `connect(dst)` constructor that the obvious name collides with) to `crates/protocol`. Handshake is Hello (credential) → Challenge (server nonce) → ChallengeResponse (agent signs the nonce) → Welcome (heartbeat interval/threshold), then periodic Heartbeat/HeartbeatAck. Control plane (`crates/control-plane/src/stream.rs`) verifies the credential via Phase 1's `verify_agent_credential`, then the challenge signature proves possession of the private key — closing the gap flagged in Phase 1 (a bearer credential alone doesn't prove the caller holds the key). `agents.last_heartbeat_at` (migration `0002`) is updated on every heartbeat; online/offline is computed at read time (`Store::list_agents`), not stored. Added a minimal unauthenticated `GET /agents` JSON endpoint (axum, separate port) as the roadmap's "basic agent list" placeholder ahead of Phase 5's real dashboard/frontend. Agent side (`crates/agent/src/stream.rs`, `src/backoff.rs`) runs the handshake, sends heartbeats on the server-provided interval, and on any disconnect retries with exponential backoff (1s → 60s cap, ±20% jitter) reusing the stored identity + credential — no re-pairing. 24 tests total: 3 new backoff unit tests, 3 new stream integration tests (happy-path handshake + online/offline transition, garbage-credential rejection, wrong-key-signs-challenge rejection) against real Postgres + real TCP, plus all 18 from Phase 1 still passing. Also end-to-end smoke-tested the real binaries: paired, watched `online` flip true on heartbeats and false ~3s after the agent was killed, then restarted the agent with only its stored credential (no token) and confirmed it reconnected without re-pairing.
- Deviations from plan:
  - **Found and fixed a real handshake deadlock during test-writing, not before.** The first implementation tried to send `Challenge` and read `ChallengeResponse` *before* returning `Response::new(stream)` from the tonic handler — but a streaming RPC's response headers (and so the client's read side) don't flush until the handler returns, so both sides blocked waiting on each other forever. Fixed by returning the response stream immediately and moving all handshake/heartbeat logic into a spawned task that reports failures as `Err(Status)` items on the stream instead of as the handler's return value. Worth calling out because it's a load-bearing pattern for any future streaming RPC in this codebase, not just this one.
  - **Deferred transport TLS again**, this time by explicit in-session decision rather than default — see updated `docs/security.md`. The roadmap's actual Phase 2 checklist doesn't list TLS; adding a dev CA/cert story was judged not worth bundling into an already-substantial phase. Proof-of-possession (the Challenge/ChallengeResponse step) *was* implemented this phase, closing the more important half of the Phase 1 gap.
  - **No shared connection registry on the control plane yet.** Each `Stream` call is handled independently; online/offline relies solely on `last_heartbeat_at`, not on which connections are currently open in memory. Intentionally deferred — Phase 3's command dispatch is what actually needs a live `agent_id -> connection` map, so building it now would be speculative.
- Open questions: see updated §8 below — heartbeat interval/threshold is now resolved; three new items added.

### Phase 3 — Complete (2026-08-21)
- Status: added `ContainerSpec`/`ContainerCommand`/`ContainerStateReport` to the protocol, as new `AgentMessage`/`ControlPlaneMessage` oneof variants (not new RPCs, per the "single persistent channel" design). Desired/observed state model: `desired_containers` (running | absent, migration `0003`) and `observed_containers` (full snapshot per report). The diff logic (`crates/control-plane/src/reconcile.rs`) is a pure function over plain structs — no DB, no protobuf — with 9 unit tests covering every convergence case. Reconciliation is triggered only when the agent sends a state report (after `Welcome`, after any command, and on the heartbeat cadence otherwise), not by an instant push when desired state changes via the HTTP API — deliberately avoids needing the connection registry Phase 2 flagged as deferred; full rationale in `docs/reconciliation.md`. Agent executes commands via `bollard` (`crates/agent/src/container.rs`): `deploy` is destructive-then-recreate (remove any existing container by that name, then create fresh) so it's correct for "missing," "wrong image," and "crashed" cases alike, not just first-time creation. Every Harbory-managed container is labeled (`harbory.managed=true`) and only label-filtered containers are ever listed/touched — critical since this dev host also runs unrelated containers. New `PUT`/`DELETE /agents/{id}/containers/{name}` and `GET /agents/{id}/containers` HTTP endpoints (still unauthenticated, still a Phase 5 dashboard placeholder) to declare desired state and inspect desired/observed. 37 tests total (13 new: 9 reconcile unit, 4 container/reconciliation integration against real Postgres + real TCP), all passing, plus all 24 from Phases 1-2 still green.
- Deviations from plan:
  - **Two real bugs found only by actually deploying a container against a live Docker daemon, not by code review or unit tests.** (1) bollard's `create_container` doesn't auto-pull a missing image the way `docker run` does — first attempt 404'd deploying `hello-world:latest`. Fixed by calling the pull API first in `deploy`, swallowing pull errors (an already-cached local-only image should still work even if the registry is unreachable) and letting `create_container`'s own error be the one that surfaces. (2) Sending a state report immediately after every command, combined with reconciling on every report, turned a persistently-failing deploy into an unbounded-rate retry loop against the Docker API — measured ~17 attempts in under 200ms in the first live test. Fixed by only sending the immediate post-command report on success; a failure is instead picked up at the next periodic report, bounding retries to the heartbeat cadence. Both are written up in detail in `docs/reconciliation.md` because the fixes (and the reasoning behind them) matter beyond just this bug.
  - **Added error-state tracking to `ContainerManager`** (not originally planned) as a consequence of fixing bug (1) above: a container that fails to even get created has no Docker-side record at all, so without this it looked identical to "nothing was ever attempted" instead of surfacing the failure. Now tracked in-memory per container name and reported as `ContainerStatus::Error`, verified live via `GET /agents/{id}/containers`.
- Open questions: see updated §8 below — one new item (retry backoff).

### Phase 4 — Complete (2026-08-21)
- Status: added `ProxyRoute`/`ProxyConfig`/`ProxyState` to the protocol as new oneof variants, reusing the exact same "report-triggered reconciliation, no connection registry, full snapshot not a delta" pattern established in Phase 3 rather than inventing a proxy-specific one. One desired route = one Nginx `server{}` block (name-based virtual hosting; v1 doesn't merge same-host routes into shared blocks). `desired_proxy_routes` has no separate absent status — presence of the row *is* desired — `DELETE` really deletes, unlike containers, because the whole config file is regenerated from the complete route set on every apply rather than patched incrementally. Convergence is detected by comparing a hash of desired routes (control plane) against a hash of last-applied routes (agent) — that hash function (`crates/protocol/src/proxy_hash.rs`) deliberately lives in the shared protocol crate so both sides run the literal same code rather than risking two independent reimplementations drifting apart. Agent-side (`crates/agent/src/proxy.rs`): validate-before-apply via `nginx -t` against the real config path (not a shadow copy, so the *effective* config — main nginx.conf plus includes — is actually what gets tested), restore-previous-content-on-failure (no reload issued, so a failed validation never affects live traffic), graceful `-s reload` never a restart, and a `tokio::sync::Mutex` around the whole sequence to make "concurrent config changes serialize, not clobber" a structural guarantee rather than an accident of the current single-threaded call path. New `PUT`/`DELETE /agents/{id}/proxy-routes/{name}` and `GET /agents/{id}/proxy-routes` HTTP endpoints. 51 tests total (14 new: 5 proxy_hash unit, 5 proxy-render unit, 4 proxy reconciliation integration against real Postgres + real TCP), all passing.
- Deviations from plan:
  - **No live end-to-end test of the actual apply/validate/reload/rollback sequence through the real running agent** — this Windows dev sandbox has no nginx installed natively, unlike every previous phase's smoke test which exercised the real compiled binaries against real infrastructure (real Postgres, real Docker). What *was* verified against a real nginx parser: the exact output of `proxy::render` (via the new `cargo run -p harbory-agent --example render_proxy_config`, piped into a scratch `nginx:alpine` container) passes `nginx -t` — confirms the template itself is syntactically valid, which is the piece most likely to have a real bug, per Phase 3's bollard lesson. The file-backup/restore and subprocess sequencing logic was reviewed carefully but not exercised live. Revisit the first time work happens on an actual Linux host with nginx installed.
  - **No TLS termination** — HTTP only, consistent with the project's other deferred-TLS decisions. Real cert provisioning is substantial enough to be its own piece of work, not a natural fit alongside building the basic validate/reload plumbing.
  - **Added a small `lib.rs` to the agent crate** (previously binary-only) purely so the new example could reuse the real `render` implementation for the live-nginx verification above, rather than needing a hand-copied approximation of it.
- Open questions: see updated §8 below — two new items (live nginx testing gap, TLS termination).

### Phase 5 — Complete (2026-08-21)
- Status: two stack decisions made with the user (frontend and auth, both explicitly deferred until this phase): **React + Vite + TypeScript** for `frontend/`, and **Supabase** (Cloud, both database and auth, migrating everything from the local `harbory-postgres` container) for accounts/login. Backend: `crates/control-plane/src/auth.rs` verifies Supabase-issued HS256 JWTs and provisions/updates the local `accounts` row on every authenticated request (`AuthenticatedAccount`, an axum extractor). Every existing HTTP endpoint now requires auth; anything scoped to an `agent_id` also requires ownership (`require_owned_agent`, 404 not 403 — same don't-leak-existence rationale used throughout this project). New endpoints: `GET /me`, `POST /pairing-tokens` (the real backend for the pairing UI, replacing the dev-only CLI for anything account-scoped), `POST /agents/{id}/revoke` (flips `agents.status`; `verify_agent_credential`, built all the way back in Phase 1, already rejects non-`'active'` agents — this is what finally lets an operator trigger that path). `frontend/`: `Login` (email/password + GitHub OAuth), `Dashboard` (agent list, pairing-token generator showing the token once with the install command, revoke), `AgentDetail` (container + proxy-route deployment forms and desired/observed tables) — all built against the existing JSON API via `@tanstack/react-query`. 67 tests total, 11 new this phase: 4 JWT-verification unit tests with synthetic tokens (`crates/control-plane/src/auth.rs`), and 7 full-router integration tests via `Router::oneshot` (`crates/control-plane/tests/http.rs`) covering missing/garbage tokens, cross-account access denial, ownership success, pairing-token issuance, and revoke — all against real local Postgres with a synthetic JWT rather than a live Supabase project. Also produced dashboard mockups (login, agent list, agent detail) as a design canvas artifact, at the user's request mid-phase — dark/technical aesthetic, Space Grotesk + JetBrains Mono.
- Deviations from plan:
  - **Deliberately no hard foreign key from `accounts.id` to Supabase's `auth.users.id`**, unlike the textbook Supabase pattern — kept as an application-level invariant instead (`Store::get_or_create_account_by_id`). A hard FK would mean `accounts` could only ever be populated against a database that actually has Supabase's `auth` schema, forcing every one of the 60 pre-existing tests (none of which exercise authentication) onto a live Supabase project just to run. Two `Store` methods coexist on purpose: `create_account` (tests/dev tooling, unscoped id) and `get_or_create_account_by_id` (the real path, id = verified JWT `sub`). Full rationale in `docs/dashboard.md`.
  - **No live end-to-end test of the actual login flow** — creating a Supabase project is an account-creation step outside what Claude Code can do on the user's behalf, so real signup/OAuth/JWT-reaches-the-real-backend has not been exercised as of this write-up, the first *full* exception after Phase 4's partial one. What was verified instead: JWT verification logic against synthetic tokens (4 unit tests), the entire HTTP auth gate through the real router with a synthetic JWT (7 integration tests, including the security-critical cross-account-denial case), and the frontend builds and its login page renders correctly (confirmed in a real browser) even with Supabase unconfigured, showing a clear setup prompt rather than crashing. Revisit once real Supabase credentials are available — see `docs/dashboard.md`'s closing section for the exact checklist.
  - **Permissive CORS** (`CorsLayer::permissive()`) on the whole HTTP API — deliberately looser than typical, judged safe specifically because auth is Bearer-token-based rather than cookie-based (a cross-origin page can trigger a request but can't make the browser attach a token it doesn't already hold). Revisit if cookie auth is ever added for anything.
- Open questions: see updated §8 below — the Phase 2 "revoked agent UX" question is now partially resolved (revocation is exposed); two new items added (live Supabase testing gap, the `accounts.id`/`auth.users.id` soft-alignment trade-off).

### Phase 6 — Complete (2026-08-21)
- Status: **audit logging** — added `AuditEventType::AgentRevoked`, logged inside `Store::revoke_agent` itself (atomic with the status flip, via `UPDATE ... RETURNING account_id`) rather than at the HTTP call site, so it's captured regardless of caller. All events the roadmap names (pairing attempts, credential mismatches, revocations) are now logged; none were missing except this one. **In-dashboard alerting** (chosen over email — see `docs/observability.md` for why, mirrors the pattern of not adding external-service dependencies without a concrete need): `GET /security-events` (account-scoped, most recent 100) plus a new `AuditEventRecord`/`is_misuse_signal` distinction in the store layer, surfaced as an "Activity" feed on the dashboard that visually flags the two events the locked-in security model (§3) actually calls misuse (`pairing_token_reuse`, `credential_fingerprint_mismatch`) differently from routine ones. **Metrics**: `GET /metrics` (Prometheus text, `metrics` + `metrics-exporter-prometheus`, `crates/control-plane/src/metrics.rs`) — pairing attempts by outcome, agent connections by outcome, a currently-connected-agents gauge (RAII-guarded so it can't drift across the several exit points in the connection loop), heartbeats received, container commands dispatched by action, proxy configs dispatched. Deliberately unauthenticated (process-global aggregates only, no per-account data — conventional for scrape endpoints). 70 tests total, 3 new (`metrics_endpoint_requires_no_auth`, `revoke_appears_in_the_account_security_feed`, `pairing_token_reuse_is_flagged_as_a_misuse_signal_in_the_security_feed`, all in `crates/control-plane/tests/http.rs`). Smoke-tested live against a real running control plane: paired a real agent, triggered a real pairing-token reuse, minted a real JWT by hand against the server's configured secret, and confirmed `/metrics` and `/security-events` both showed exactly the expected values — not just synthetic-token unit tests this time.
- Deviations from plan:
  - **No email/webhook alerting built** — the roadmap explicitly allows "email or in-dashboard," and every external service integrated so far (Supabase in Phase 5) came with real setup friction (an account only the user could create); adding email would mean a *third* such dependency for a channel the roadmap treats as optional. Revisit if in-dashboard alerting proves insufficient in practice.
  - **No generic HTTP request-count/latency metrics** — only domain-level events are instrumented (pairing, connections, heartbeats, dispatches), which is what "agent and control plane health" actually calls for; a generic Axum middleware layer can be layered on later without touching this instrumentation.
- **This completes the roadmap in §5.** Later phases (multi-region control plane, HA, broader runtime support) are explicitly out of scope per §1 and deferred indefinitely, not queued as "Phase 7." Remaining work from here is the accumulated open-questions list below — most notably: live Supabase credentials still haven't been provided, so the real login flow has never been exercised end-to-end; transport is still plaintext h2c; and the live nginx apply/reload path was never tested against a real Linux host. None of these block the roadmap's stated v1 scope, but any of them would matter before a real deployment.

---
## 8. Open Design Questions (revisit before or during relevant phase)
- ~~Whether the long-lived credential is a signed JWT or a client cert (mTLS)~~ — **Resolved in Phase 1**: Ed25519-signed token (not JWT, not mTLS). See `docs/security.md`.
- ~~Database schema specifics for `agents`, `accounts`, `pairing_tokens`, `audit_log` tables~~ — **Resolved in Phase 1**: see `docs/database.md` and `crates/control-plane/migrations/0001_init.sql`. `accounts` is intentionally minimal pending Phase 5's real auth system.
- ~~Exact heartbeat interval and missed-heartbeat threshold before marking "offline"~~ — **Resolved in Phase 2**: 10s interval, offline after 3 missed (30s). See `docs/connection-lifecycle.md`.
- **New (Phase 1):** control-plane signing-key persistence assumes a single control-plane process reading one local key file (`CONTROL_PLANE_SIGNING_KEY_PATH`). Multi-instance control planes (even single-region, e.g. behind a load balancer) will need a shared key store instead of a local file — revisit before any horizontal scaling of the control plane, even though multi-region HA itself stays out of scope per §1.
- **New (Phase 2):** transport is still plaintext h2c — deferred twice now (Phase 1 default, Phase 2 explicit decision). Should get a real answer before any deployment outside local dev; not urgent while everything runs on localhost.
- **Partially resolved in Phase 5:** a revoked agent's stream connect fails (`verify_agent_credential` returns `Revoked`), and revocation is now exposed (`POST /agents/{id}/revoke`, wired into the dashboard). Still open: the agent's reconnect loop still can't distinguish "revoked" from a transient failure — it backs off and retries forever rather than surfacing "you need to re-pair." No decision has been made about what a revoked agent process should do (exit? keep retrying quietly? both, configurably?) — the control-plane side of revocation is done, the agent-side UX for it isn't.
- How command results/errors are surfaced back to the dashboard in real time (same stream vs. separate query) — still open, Phase 3+.
- **New (Phase 3):** a persistently-failing container deploy (bad image, bad credentials, etc.) retries forever at the heartbeat cadence with no backoff — confirmed live to be stable rather than accelerating, but it will keep hammering a registry that's genuinely down. Revisit if this becomes a real problem; something like Kubernetes' `ImagePullBackOff` is the shape of the eventual fix. See `docs/reconciliation.md`.
- **New (Phase 3):** whether/when to build the connection registry (`agent_id -> live connection`) that would let desired-state changes push to a connected agent instantly instead of waiting for its next report. Deferred again this phase — the report-triggered design turned out not to need it — but flagged in case the up-to-one-heartbeat-interval latency stops being acceptable.
- **New (Phase 4):** the proxy apply/validate/reload/rollback sequence (`crates/agent/src/proxy.rs`) has not been exercised live against a real nginx process — no nginx available in this dev sandbox. Only the rendered config's syntax was verified (against a real nginx parser in a scratch container). Run a real end-to-end test the first time this code touches an actual Linux host with nginx installed, before trusting it in anything resembling production.
- **New (Phase 4):** no TLS termination in proxy routes — HTTP only, same deferred-TLS pattern as the control-plane/agent transport itself (`docs/security.md`). Revisit alongside that transport-TLS decision, or independently if a real deployment needs HTTPS sooner.
- **Still open (touched again in Phase 6):** the real login flow (signup, OAuth, a genuine Supabase-issued JWT reaching the real backend) still hasn't been exercised — creating a Supabase project needs an account only the user can create. Phase 6 did go one step further than Phase 5's synthetic-token unit tests: a hand-minted JWT (same HS256 shape Supabase produces) was sent to the *actual running compiled server process* and correctly authenticated, provisioned an account, and drove `/security-events`/`/agents/{id}/revoke` for real. That proves the backend's auth logic works against a live process, not just in-test — but it's still not a real Supabase project, so signup/OAuth/session-refresh behavior remains unverified. Do the real version once credentials are in hand — see `docs/dashboard.md`'s setup checklist.
- **New (Phase 5):** `accounts.id` is only ever aligned with Supabase's `auth.users.id` by application code (`get_or_create_account_by_id`), not a database foreign key — a deliberate trade-off to keep the pre-existing test suite running against local Postgres rather than forcing it onto a live Supabase project. Revisit if this soft alignment ever causes a real data-integrity problem (e.g. a Supabase user deleted out from under an `accounts` row with no cascade to clean it up) — full rationale in `docs/dashboard.md`.
- **New (Phase 6):** no email/webhook alerting for misuse signals — in-dashboard only. Revisit if that proves insufficient once this is actually used day-to-day (e.g. an operator who doesn't have the dashboard open when a credential mismatch fires). See `docs/observability.md`.
- **New (Phase 6):** metrics cover domain-level events only (pairing, connections, heartbeats, dispatches), not generic HTTP request counts/latency, and `PrometheusHandle` state resets on every control-plane restart (in-process only, no persistence). Both are fine for a single-instance control plane; revisit together with the signing-key multi-instance question above if the control plane is ever horizontally scaled.
