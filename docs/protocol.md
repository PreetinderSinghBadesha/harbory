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
    ContainerStateReport state_report = 4;  // Phase 3
    ProxyState proxy_state = 5;              // Phase 4
  }
}

message ControlPlaneMessage {
  oneof payload {
    Challenge challenge = 1;
    Welcome welcome = 2;
    HeartbeatAck heartbeat_ack = 3;
    ContainerCommand command = 4;  // Phase 3
    ProxyConfig proxy_config = 5;   // Phase 4
  }
}
```

`Hello.credential` is the opaque credential from `RegisterResponse`.
`Welcome` carries the heartbeat interval and missed-heartbeat threshold the
control plane wants this agent to use — not hardcoded agent-side, so it can
change without an agent redeploy.

## Container commands and state (Phase 3)

```proto
message ContainerSpec {
  string name = 1;
  string image = 2;
  repeated string env = 3;
  repeated PortMapping ports = 4;
  repeated string command = 5;  // empty = image's own ENTRYPOINT/CMD
}

message ContainerCommand {
  oneof action {
    ContainerSpec deploy = 1;  // create-or-replace, idempotent
    string stop = 2;           // container name — see note below
    string remove = 3;         // container name
  }
}

message ContainerStateReport {
  repeated ContainerState containers = 1;  // full snapshot, not a delta
}
```

`ContainerCommand.stop` exists in the wire format for a possible future
manual/dashboard-triggered stop, but the automatic reconciler
(`crates/control-plane/src/reconcile.rs`) never emits it — v1 desired
state is only "running" or "absent," there's no "stopped but present"
state to converge toward. Full reconciliation design, including why
commands are dispatched only in response to a state report rather than
pushed the instant desired state changes:
[`reconciliation.md`](reconciliation.md).

## Proxy commands and state (Phase 4)

```proto
message ProxyRoute {
  string name = 1;
  string server_name = 2;   // Host header to match; empty = catch-all
  uint32 listen_port = 3;
  string path_prefix = 4;   // default "/"
  string upstream_host = 5;
  uint32 upstream_port = 6;
}

message ProxyConfig {
  repeated ProxyRoute routes = 1;  // full desired set, not a delta
}

message ProxyState {
  bytes applied_hash = 1;  // see proxy_hash below; empty = nothing applied yet
  string error = 2;
}
```

One route = one Nginx `server{}` block; there's no merging of routes that
share a `server_name`+`listen_port` into a single block with multiple
`location`s. `ProxyConfig` is always the complete desired route set, same
"full snapshot, not a delta" rationale as `ContainerStateReport`. Full
design — including the validate/reload/rollback sequence and why
reconciliation is triggered by `ProxyState` reports rather than pushed
instantly: [`proxy-management.md`](proxy-management.md).

**`applied_hash` must be computed identically by both sides**, or
convergence detection breaks. The hash function
(`hash_routes`) lives in `crates/protocol/src/proxy_hash.rs` — in this
crate, not duplicated on each side — specifically so there's exactly one
implementation both the control plane and the agent call.

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

## On-demand inspection: Images, Networks, Volumes, Docker Containers (post-Phase-6 features)

The dashboard's Images / Networks / Volumes / Containers tabs are read-only
views backed by a *fire-and-forget* request/response pair over the same
persistent stream — not new RPCs. The control plane inserts a pending
`oneshot` channel keyed by a `request_id`, writes a `*Request` onto the
stream, and returns the corresponding HTTP response as soon as the agent
replies with the matching `*Response`. No result is ever stored; the reply
is resolved in memory and dropped once the HTTP handler reads it.

Because it's fire-and-forget, a request made while the agent is offline
is rejected immediately with `503` — there's no queueing against an
unknown delivery time.

```proto
// Each resource follows the exact same shape.

message ImagesRequest   { string request_id = 1; string remove_image_id = 2; }
message ImagesResponse  { string request_id = 1; repeated ImageInfo images = 2; string error = 3; }

message NetworksRequest { string request_id = 1; string remove_network_name = 2; }
message NetworksResponse{ string request_id = 1; repeated NetworkInfo networks = 2; string error = 3; }

message VolumesRequest  { string request_id = 1; string remove_volume_name = 2; }
message VolumesResponse { string request_id = 1; repeated VolumeInfo volumes = 2; string error = 3; }
```

Each `*Request` is a new oneof variant on `ControlPlaneMessage` and each
`*Response` on `AgentMessage` (e.g. `volumes_request = 12` /
`volumes_response = 12`). A `DELETE` writes a non-empty
`remove_<resource>_id/name` so the agent removes that entity before
re-listing; a `GET` sends an empty removal field.

The `in_use` flags on `ImageInfo` and `VolumeInfo` are computed by listing
all containers (running *and* stopped) on the host and checking each one's
image id / mounted volume names — a container holds its image id and its
volume mounts even when stopped, so an in-use entity's removal is disabled
in the UI rather than failing on a Docker conflict error.

```proto
message VolumeInfo {
  string name = 1;
  string driver = 2;
  string mountpoint = 3;
  int64 created_at = 4;  // unix seconds
  bool in_use = 5;       // mounted by at least one running or stopped container
}
```

The shared container-inspection helper that computes these sets lives in
`crates/agent/src/docker_inspect.rs`, used by both `ImagesManager` and
`VolumesManager`.

| Dashboard tab | Agent handler | HTTP endpoint |
|---|---|---|
| Images | `crates/agent/src/images.rs` (`ImagesManager`) | `GET/DELETE /agents/:id/images[/:image_id]` |
| Networks | `crates/agent/src/networks.rs` (`NetworksManager`) | `GET/DELETE /agents/:id/networks[/:network_id]` |
| Volumes | `crates/agent/src/volumes.rs` (`VolumesManager`) | `GET/DELETE /agents/:id/volumes[/:volume_name]` |

Volumes visibility is purely read-only listing + manual removal of
*existing* host volumes (compose-stack volumes, `docker volume create`,
leftovers from removed containers, etc.) — it does not provision or attach
persistent storage to deployed containers, which is a separate concern.

## Not yet defined (later phases)

Nothing yet — Phases 1-4's message types are all defined above. Later
phases (dashboard real-time updates, etc.) may add more.
