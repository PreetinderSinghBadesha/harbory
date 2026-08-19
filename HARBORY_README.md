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
├── frontend/              # web dashboard (decide stack in Phase 5)
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

---
## 8. Open Design Questions (revisit before or during relevant phase)
- ~~Whether the long-lived credential is a signed JWT or a client cert (mTLS)~~ — **Resolved in Phase 1**: Ed25519-signed token (not JWT, not mTLS). See `docs/security.md`.
- ~~Database schema specifics for `agents`, `accounts`, `pairing_tokens`, `audit_log` tables~~ — **Resolved in Phase 1**: see `docs/database.md` and `crates/control-plane/migrations/0001_init.sql`. `accounts` is intentionally minimal pending Phase 5's real auth system.
- **New (Phase 1):** control-plane signing-key persistence assumes a single control-plane process reading one local key file (`CONTROL_PLANE_SIGNING_KEY_PATH`). Multi-instance control planes (even single-region, e.g. behind a load balancer) will need a shared key store instead of a local file — revisit before any horizontal scaling of the control plane, even though multi-region HA itself stays out of scope per §1.
- Exact heartbeat interval and missed-heartbeat threshold before marking "offline" — still open, Phase 2.
- How command results/errors are surfaced back to the dashboard in real time (same stream vs. separate query) — still open, Phase 3+.
