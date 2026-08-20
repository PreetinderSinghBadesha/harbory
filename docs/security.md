# Security: token & credential lifecycle (Phase 1)

This finalizes the design sketched in `HARBORY_README.md` §3, resolving the
open questions that were explicitly deferred to Phase 1.

## Decision: signed token, not mTLS

`HARBORY_README.md` noted "leaning mTLS per earlier discussion, confirm in
Phase 1." Phase 1 confirmed the **other** option the security model already
allowed for ("a signed credential (cert *or* signed token)"): an
Ed25519-signed token, not a client certificate / CA.

**Why:** mTLS requires the control plane to run a CA, issue and rotate
per-agent client certs, and wire cert-based auth into the rustls layer of
the gRPC stream. A signed token gets the same binding
(`agent_id ↔ account_id ↔ public_key_fingerprint`) with far less
infrastructure: no CA, no cert lifecycle, and revocation/expiry are plain
database state instead of CRL/OCSP machinery. This was a deliberate
trade-off, decided with the user in this session — flagging it here per the
"don't deviate without discussion" rule in §0, even though the doc already
listed it as an open question rather than a lock-in.

**Consequence:** the architecture diagram's "mTLS/token-auth" transport is
implemented as server-side TLS (rustls, confidentiality/integrity only) +
application-layer signed-token auth — not mutual TLS. Server-side TLS itself
is not yet wired up (see "Deferred to Phase 2" below); Phase 1's `Register`
RPC runs over plaintext h2c for local testing.

## Pairing token lifecycle

1. Control plane generates 32 random bytes, encodes as `hbp_<base64url>`,
   and stores only `SHA256(token)` — same rationale as password/API-key
   hashing, so a DB read alone can't produce a usable token.
2. Token is single-use and tied to one `account_id`, with a caller-supplied
   TTL (10 minutes in the example CLI).
3. Consumption is race-safe: `register_agent` takes a row lock
   (`SELECT ... FOR UPDATE`) on the token inside a transaction before
   validating and consuming it, so two concurrent registrations with the
   same token cannot both succeed. Covered by
   `concurrent_registration_with_same_token_only_succeeds_once` in
   `crates/control-plane/tests/registration.rs`.
4. Reuse of an already-consumed token, and use of an expired token, are
   both rejected with the *same* wire-visible error as an unknown token
   (see `protocol.md`), but are logged distinctly to `audit_log` internally.
   Per §3's "notify account owner" requirement: the audit row is written
   now (Phase 1); actual delivery (email/dashboard alert) is Phase 6 scope,
   consistent with the roadmap's "Alerting/notifications" line item.

## Credential issuance

On successful registration, in the same transaction as consuming the token
and creating the `agents` row, the control plane builds:

```
CredentialPayload { agent_id, account_id, public_key_fingerprint, issued_at }
```

and signs it with its own Ed25519 keypair (`sign_credential`,
`crates/common/src/credential.rs`). The signing keypair is loaded from
`CONTROL_PLANE_SIGNING_KEY_PATH` (generated and persisted on first run if
absent — `Keypair::load_or_generate`) so a control-plane restart doesn't
invalidate every credential it already issued.

## Credential verification (`Store::verify_agent_credential`)

Implemented and tested now; not yet wired into a live connection because
the persistent stream doesn't exist until Phase 2. What it checks:

1. **Signature** — `verify_credential` confirms the credential's payload
   was actually signed by the control plane's key. Failure →
   `VerifyCredentialError::Invalid`.
2. **Agent still known** — looks up `agent_id` from the payload.
   Not found → `UnknownAgent`.
3. **Not revoked** — `agents.status == 'active'`. Otherwise → `Revoked`.
4. **Fingerprint match** — the fingerprint inside the credential must equal
   `SHA256(agents.public_key)` on file for that `agent_id`. Mismatch is the
   §3 "possible compromise" case: rejected as `FingerprintMismatch` *and*
   logged to `audit_log` as `credential_fingerprint_mismatch` (the
   "stronger alert" §3 calls for — again, delivery mechanism is Phase 6).

### Implemented in Phase 2: proof of possession

Presenting a valid, unrevoked, fingerprint-matching credential proves the
credential is genuine — it does not by itself prove the *caller* holds the
matching private key (a stolen credential file would pass all four checks
above). `AgentStreamService.Stream` (`crates/control-plane/src/stream.rs`)
closes this: after `verify_agent_credential` succeeds, the control plane
sends a fresh random 32-byte nonce (`Challenge`), the agent signs it with
its identity private key (`ChallengeResponse`), and the control plane
verifies that signature against the *stored* public key for that
`agent_id` (not a key embedded in the credential — the credential only
carries a fingerprint) before sending `Welcome` and treating the stream as
authenticated. A 10-second timeout on the response prevents a slow/hung
client from holding a half-open connection. Full sequence diagram in
`docs/connection-lifecycle.md`.

### Still deferred: transport TLS

Wiring rustls into the tonic server (and pinning/verifying the control
plane's TLS identity on the agent side) was explicitly deferred again in
Phase 2 — the roadmap's Phase 2 checklist doesn't call for it, and challenge
signing above already closes the more security-critical gap (proving
private-key possession) independent of transport encryption. The stream
still runs over plaintext h2c, so it remains vulnerable to a network-level
eavesdropper/MITM until this is picked up — tracked as an open question in
`HARBORY_README.md` §8.

## Revocation

Not yet exposed via any RPC or UI (Phase 5). `agents.status` already
supports `'revoked'` in the schema (see `docs/database.md`), and
`verify_agent_credential` already rejects revoked agents — only the
dashboard action to flip the status is missing.
