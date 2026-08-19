# Protocol (Phase 1)

Wire protocol lives in [`crates/protocol`](../crates/protocol), generated from
[`proto/harbory.proto`](../crates/protocol/proto/harbory.proto) via `tonic-build`.
`protoc` is vendored at build time (`protoc-bin-vendored`) so no system install
is required.

Phase 1 defines exactly one service, `PairingService`, used once per agent to
bootstrap trust. The persistent bi-directional stream (heartbeats, commands)
is a Phase 2/3 addition to this same crate — it is not in this file yet
because Phase 1 explicitly should not build ahead of its scope.

## `PairingService.Register`

Unary RPC. Called once by a freshly-booted agent that already has a local
Ed25519 keypair and a pairing token copied from the dashboard.

```proto
rpc Register(RegisterRequest) returns (RegisterResponse);

message RegisterRequest {
  string pairing_token = 1;
  bytes public_key = 2;   // 32-byte Ed25519 public key
}

message RegisterResponse {
  string agent_id = 1;
  string account_id = 2;
  bytes credential = 3;                 // opaque, see below
  bytes control_plane_public_key = 4;   // 32-byte Ed25519 public key
}
```

### Errors

All rejection paths (unknown token, already-consumed token, expired token)
return the same `PermissionDenied` status with the same message
(`"invalid or expired pairing token"`). This is deliberate: the wire
response must not let a caller distinguish "never existed" from "already
used" from "expired" — that distinction only exists in the audit log
(`account_id` scoped, not attacker-visible). See
[`security.md`](security.md) for the full misuse-detection rationale.

## Credential format

The `credential` bytes returned by `Register` are opaque to the wire
protocol — no message type wraps them, they're a flat `bytes` field. Layout
(defined in [`harbory-common`](../crates/common/src/credential.rs), not in
`.proto`, since only the two Rust sides ever need to parse it):

```
[ agent_id: 16 bytes | account_id: 16 bytes | fingerprint: 32 bytes | issued_at: i64 LE ] || [ signature: 64 bytes ]
  \_______________________________ payload (72 bytes) _______________________________/
```

`fingerprint` is `SHA-256(agent's 32-byte Ed25519 public key)`. `signature`
is the control plane's Ed25519 signature over the 72-byte payload. See
`docs/security.md` for what's checked and when.

## Not yet defined (later phases)

- Persistent stream service (heartbeats, command dispatch, state reporting) — Phase 2/3.
- Container command messages — Phase 3.
- Proxy config command messages — Phase 4.
