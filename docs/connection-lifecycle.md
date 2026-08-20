# Connection lifecycle (Phase 2)

The persistent `AgentStreamService.Stream` RPC (named `Stream`, not
`Connect` — tonic's generated client already has a `connect(dst)`
constructor and the names collided) carries the handshake and heartbeats.
Command dispatch and state reporting (Phase 3+) reuse this same stream
rather than adding new RPCs, per the architecture doc's "single persistent
authenticated channel" design.

## Handshake

```
agent                                   control plane
  |-- Hello { credential } ------------------>|
  |                                            |  verify_agent_credential:
  |                                            |  signature, known agent,
  |                                            |  not revoked, fingerprint
  |                                            |  match (see security.md)
  |<----------------- Challenge { nonce } -----|
  |-- ChallengeResponse { sign(nonce) } ------>|
  |                                            |  verify signature against
  |                                            |  the agent's stored public
  |                                            |  key (proof of possession)
  |<------------------- Welcome { interval } --|
  |-- Heartbeat --> ... (every interval) ----->|
  |<-- HeartbeatAck -- ... -------------------|
```

Rejecting at any step closes the stream with `Status::unauthenticated` (or
`invalid_argument`/`deadline_exceeded` for malformed/slow clients) — no
message distinguishes *why*, same rationale as the pairing RPC in
`docs/protocol.md`.

**Why the challenge step exists:** the credential from `Register` is a
bearer token — anyone holding the bytes can present it. `verify_agent_credential`
alone (checked at `Hello`) proves the credential is genuine and unrevoked,
not that the *caller* holds the matching private key. The
`Challenge`/`ChallengeResponse` round trip closes that gap: the control
plane picks a fresh random nonce per connection attempt and the agent must
sign it with its identity key. A stolen credential file without the
private key fails here. This was flagged as deferred work in Phase 1's
`docs/security.md` and is now implemented in
`crates/control-plane/src/stream.rs`.

**Timeout:** the control plane waits at most 10 seconds for
`ChallengeResponse` (`CHALLENGE_RESPONSE_TIMEOUT`) before closing the
stream, so a slow or hung client can't hold a half-open connection
indefinitely.

## Heartbeat interval and offline threshold

**Decision (resolves the open question in `HARBORY_README.md` §8):**
10-second heartbeat interval, offline after 3 consecutive misses (30
seconds with no heartbeat). Defaults live in
`crates/control-plane/src/main.rs`
(`DEFAULT_HEARTBEAT_INTERVAL_SECONDS`/`DEFAULT_MISSED_HEARTBEAT_THRESHOLD`),
overridable via `HEARTBEAT_INTERVAL_SECONDS`/`MISSED_HEARTBEAT_THRESHOLD`
env vars, and communicated to each agent in `Welcome` rather than hardcoded
agent-side — so the interval can change without an agent redeploy.

**Why 10s/3:** a compromise between responsiveness (up to 30s to notice an
agent is gone) and load (one small message every 10s per agent, one DB
write per heartbeat). No load testing has been done — revisit if/when
agent counts get large enough for per-heartbeat DB writes to matter (see
"Not done" below).

**How online/offline is computed:** `agents.last_heartbeat_at` is updated
on every heartbeat (`Store::record_heartbeat`); "online" is *computed*, not
stored — `now() - last_heartbeat_at < threshold`, done at query time in
`Store::list_agents` (used by the `/agents` HTTP endpoint). No background
sweeper flips a status column. This was a deliberate simplification over a
sweeper: a computed value can't drift out of sync with reality, and it's
trivial to test (see `handshake_succeeds_and_heartbeat_marks_agent_online`
in `crates/control-plane/tests/stream.rs`, which asserts online with a
generous threshold and offline with a threshold of `0`).

## Reconnect / backoff (agent side)

`crates/agent/src/backoff.rs`: exponential backoff starting at 1s,
doubling each failed attempt, capped at 60s, with ±20% jitter to avoid a
thundering herd of agents all reconnecting on the same schedule after a
control-plane restart. Resets to 1s after any successful `Welcome`.

No re-pairing on disconnect — the agent's main loop
(`crates/agent/src/main.rs`) just calls `stream::run_stream` again with the
same stored identity keypair and credential, per the security model's
"transient disconnects don't require re-pairing" rule. A brand-new pairing
token is only needed if the credential itself stops working (e.g. the
agent was revoked — `verify_agent_credential` then returns `Revoked`, which
surfaces to the agent as a stream-open failure that backoff will retry
forever without success; there's no revocation-aware "stop retrying and
prompt for re-pairing" UX yet, since revocation isn't exposed anywhere
until Phase 5).

## Not done in Phase 2 (intentionally)

- **Transport TLS.** Still plaintext h2c — deferred again, this time by
  explicit decision in this session rather than by default. See
  `docs/security.md`. Agent authentication still happens at the
  application layer (credential + challenge/response above); deferring TLS
  means the *transport* isn't yet hardened against a network-level
  eavesdropper/MITM.
- **A live connection registry on the control plane.** Each `Stream` call
  is handled independently; there's no shared map of `agent_id -> active
  connection` yet. Not needed until Phase 3 needs to push a command to a
  specific connected agent — added then, not speculatively now.
- **Explicit offline-transition events / alerting.** Online/offline is
  computed on read, not pushed. Phase 6 observability will likely want an
  explicit "agent went offline" signal for alerting; that's a consumer of
  `last_heartbeat_at`, not a reason to change how it's stored now.
