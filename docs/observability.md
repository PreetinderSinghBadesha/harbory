# Observability (Phase 6)

## Audit logging

`audit_log` (schema since Phase 1, `crates/control-plane/migrations/0001_init.sql`) has carried security-relevant events from the start; this phase completes the set the roadmap calls for ("pairing attempts, credential mismatches, revocations") and makes them actually visible.

Event types (`AuditEventType`, `crates/control-plane/src/store/audit.rs`):

| Event | Since | Misuse signal? |
|---|---|---|
| `pairing_success` | Phase 1 | no |
| `pairing_token_reuse` | Phase 1 | **yes** |
| `pairing_token_expired` | Phase 1 | no |
| `credential_fingerprint_mismatch` | Phase 2 | **yes** |
| `agent_revoked` | Phase 6 | no |

"Misuse signal" (`is_misuse_signal` in `crates/control-plane/src/store/audit.rs`) is the roadmap's §3 distinction: token reuse and fingerprint mismatch are the two events the locked-in security model explicitly calls "reject + notify account owner" / "reject + stronger alert" — everything else is routine activity, not a sign of compromise. The dashboard's activity feed highlights only these two differently (see below); it still shows the rest for a full audit trail.

`agent_revoked` is new this phase: `Store::revoke_agent` now logs it in the same call that flips `agents.status`, so revocation (an operator action, not misuse) shows up in the same feed as the misuse signals — useful for "who revoked what, when," not itself something to alert on.

## In-dashboard alerting, not email

The roadmap allows either ("Alerting/notifications (email or in-dashboard) for misuse signals"). This phase built in-dashboard: `GET /security-events` (`crates/control-plane/src/http.rs`, authenticated + account-scoped) returns the most recent 100 events for the caller's account, and `frontend/src/pages/Dashboard.tsx` renders them as an "Activity" feed, visually flagging `is_misuse_signal` rows.

**Why not email:** every external service this project has integrated (Supabase in Phase 5) needed the user to create an account and hand over credentials — email delivery would mean a *third* external dependency (an SMTP relay or a transactional email API like Resend/SendGrid) with the same friction, for a notification channel the roadmap already treats as optional. In-dashboard alerting needed nothing beyond what Phases 1-5 already built (the audit log, the auth system, the dashboard shell). Revisit if real usage shows people miss alerts by not having the dashboard open — that's the point where a genuinely async channel (email, or a webhook) earns its cost.

## Metrics

`GET /metrics`, Prometheus text format, via `metrics` + `metrics-exporter-prometheus` (`crates/control-plane/src/metrics.rs`). Counters/gauges, instrumented at their call sites in `grpc.rs` and `stream.rs`:

- `harbory_pairing_attempts_total{outcome}` — `success` / `invalid_token` / `token_already_used` / `token_expired` / `db_error`.
- `harbory_agent_connections_total{outcome}` — `success` / `invalid_credential` / `invalid_challenge_signature`.
- `harbory_agents_connected` (gauge) — incremented when a stream authenticates, decremented via an RAII guard (`ConnectedAgentGuard`) on every exit from the connection-handling loop, so it can't drift from double-counting or a missed decrement on one of several `break` points.
- `harbory_heartbeats_received_total`.
- `harbory_container_commands_dispatched_total{action}` — `deploy` / `remove`.
- `harbory_proxy_configs_dispatched_total`.

**Why `/metrics` has no auth**, unlike every other route: it exposes process-global aggregates only — no per-account or per-agent data is queryable through it (there's no way to ask it "how many of *my* agents are connected," only the fleet-wide total). Prometheus scrape endpoints are conventionally protected at the network layer (firewall rules, an internal-only listener, a reverse-proxy scrape allowlist) rather than per-request application auth, since the scraper is infrastructure, not a dashboard user. Verified live against a real running control plane (not just the unit-level `metrics_endpoint_requires_no_auth` test): paired a real agent, triggered a real pairing-token reuse, and confirmed `/metrics` showed the exact expected counts (`pairing_attempts_total{outcome="success"} 1`, `{outcome="token_already_used"} 1`, `agents_connected 1`, `heartbeats_received_total 8`, etc.).

## Structured logging

Already established since Phase 1 via the `tracing` crate — every phase's control-plane and agent code logs through it (`tracing::info!`/`warn!`/`error!`/`debug!`), not `println!`. This phase didn't change that convention, just formalizes it here: prefer structured fields (`tracing::warn!(%agent_id, ?err, "...")`) over interpolating values into the message string, so log lines stay greppable/parseable by field name. No log aggregation/shipping is set up — `tracing_subscriber::fmt` writes to stdout, same as every previous phase; wiring that to a real log platform is deployment-specific and out of scope here.

## Not done in Phase 6 (intentionally)

- **No email/webhook alerting** — see above; revisit if in-dashboard proves insufficient.
- **No metrics for HTTP request counts/latency** — only the domain-level events (pairing, connections, heartbeats, dispatches) are instrumented, not a generic Axum middleware layer counting every route by status code. Those domain events are what the roadmap's "agent and control plane health" actually calls for; generic HTTP metrics can be added later without touching this instrumentation if they turn out to be needed.
- **No metrics persistence/retention story** — `PrometheusHandle` holds everything in-process; a restart resets all counters to zero. Fine for a single-process, single-instance control plane (matches the existing signing-key-persistence open question about not yet supporting multiple control-plane instances); revisit together if/when that changes.
