# Protocol

Wire protocol lives in [`crates/protocol`](../crates/protocol), generated from
[`proto/harbory.proto`](../crates/protocol/proto/harbory.proto) via `tonic-build`.
`protoc` is vendored at build time (`protoc-bin-vendored`) so no system install
is required.

Two services so far: `PairingService` (Phase 1, one-shot bootstrap) and
`AgentStreamService` (Phase 2, the persistent connection). Command dispatch
and state reporting (Phase 3+) will extend `AgentStreamService`'s existing
`AgentMessage`/`ControlPlaneMessage` oneofs with new variants rather than
adding new RPCs — that's the whole point of the "single persistent
authenticated channel" design.

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

## `AgentStreamService.Stream`

Bidirectional streaming RPC — named `Stream`, not `Connect`: tonic's
generated client already has an associated `connect(dst)` constructor and
the names collided (`E0592: duplicate definitions with name 'connect'`).
Carries the connect-time handshake and, once authenticated, periodic
heartbeats. Full handshake sequence, timeout, and rationale for the
challenge/response step: [`connection-lifecycle.md`](connection-lifecycle.md).

```proto
rpc Stream(stream AgentMessage) returns (stream ControlPlaneMessage);

message AgentMessage {
  oneof payload {
    Hello hello = 1;
    ChallengeResponse challenge_response = 2;
    Heartbeat heartbeat = 3;
  }
}

message ControlPlaneMessage {
  oneof payload {
    Challenge challenge = 1;
    Welcome welcome = 2;
    HeartbeatAck heartbeat_ack = 3;
  }
}
```

`Hello.credential` is the opaque credential from `RegisterResponse`.
`Welcome` carries the heartbeat interval and missed-heartbeat threshold the
control plane wants this agent to use — not hardcoded agent-side, so it can
change without an agent redeploy.

### A gotcha worth remembering for future streaming RPCs

The first implementation of this handler tried to send `Challenge` and read
`ChallengeResponse` *before* returning `Response::new(stream)` — but a
streaming RPC's response headers (and so the client's read side) don't
flush until the handler actually returns. Both sides ended up blocked
waiting on each other forever. Fix: return the response stream immediately,
and do all handshake/heartbeat logic in a spawned task that reports
failures as `Err(Status)` *items* on the stream rather than as the
handler's own return value. See `crates/control-plane/src/stream.rs`
(`drive_connection`) for the pattern.

## Not yet defined (later phases)

- Command dispatch and state reporting, as new `AgentMessage`/`ControlPlaneMessage` variants — Phase 3.
- Container command messages — Phase 3.
- Proxy config command messages — Phase 4.
