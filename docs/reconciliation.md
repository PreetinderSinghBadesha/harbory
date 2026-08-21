# Reconciliation (Phase 3)

The roadmap calls this "the reconciliation foundation — don't skip," so
it's worth documenting precisely, not just as code comments.

## Model

Deliberately narrow, per the project's stated non-goals (§1: no arbitrary
workload scheduling). A desired container has exactly two possible
statuses:

- **Running** — should exist, with a given image/env/ports/command.
- **Absent** — should not exist.

No "stopped but present," no restart policies, no replica counts. If a
richer model turns out to be needed, it's an additive change to
`desired_containers.desired_status` and `reconcile::DesiredStatus`, not a
redesign.

## The pure diff function

`crates/control-plane/src/reconcile.rs::diff(desired, observed) -> Vec<Action>`
takes plain Rust structs (no DB, no protobuf) and returns the actions
needed to converge. `Action` is just `Deploy(DesiredContainer)` or
`Remove(String)` — no `Stop` action, even though the wire protocol has a
`ContainerCommand::stop` variant for a possible future manual/dashboard
"stop without removing" trigger. The automatic reconciler doesn't need it:
"stopped" isn't a desired state in this model, so nothing ever computes a
diff that wants it.

**Convergence rules**, matched by the unit tests in that file:

| Desired | Observed | Action |
|---|---|---|
| Running | missing / stopped / errored / wrong image | `Deploy` |
| Running | running, correct image | none (converged) |
| Absent | missing / already removed | none (converged) |
| Absent | exists in any other state | `Remove` |
| *(no desired row)* | exists in any state | none — see below |

**Why "no desired row" is never touched:** an observed container with no
matching desired entry means nobody has expressed an opinion about it —
possibly manual intervention, possibly a desired row that was deleted
outright rather than set to `absent`. The safe default is to leave it
alone rather than guess it should be removed. (This is distinct from, and
layered under, the host-safety filtering described below — that filtering
means Harbory's reconciler never even *sees* non-Harbory containers in the
first place.)

## `Deploy` is idempotent by being destructive

`ContainerManager::deploy` (`crates/agent/src/container.rs`) always removes
any existing container by that name first, then creates fresh — rather
than trying to inspect and patch an existing container in place. This
means the same `Deploy` action is correct whether the container doesn't
exist yet, is running the wrong image, crashed, or is stuck in a weird
state — the reconciler doesn't need separate logic for each case, and
there's no in-place-update code path to get subtly wrong.

## Host safety: label-scoped, not name-scoped

This control plane's dev environment (and presumably most real ones) runs
Docker containers Harbory doesn't own. `ContainerManager` only ever lists,
reports, or removes containers carrying the label `harbory.managed=true`
(set by `deploy`, filtered on by `list_state`) — an unfiltered
`list_containers`/`remove_container` sweep is exactly the kind of mistake
that would be catastrophic on a shared host, so it's structurally
prevented rather than merely avoided by convention. The actual Docker
container name is also prefixed (`harbory-<logical-name>`), belt-and-braces
against colliding with an unrelated container that happens to share a
name.

## How reconciliation is triggered

**Decision:** only when the agent sends a `ContainerStateReport` — not
immediately when desired state changes via the HTTP API, and not via a
control-plane-held registry of live connections that could push to a
specific agent on demand.

**Why:** the alternative (push the moment `PUT /agents/{id}/containers/{name}`
is called) requires the control plane to hold a live `agent_id ->
connection` map so it can find and write to the right in-flight stream
from an unrelated HTTP request handler. That's a real piece of shared
state with its own concurrency considerations, and nothing in Phase 1 or 2
built it — Phase 2's docs explicitly deferred it as "needed once Phase 3's
command dispatch needs to push to a specific connection." Having reached
that point, the report-triggered design below turned out not to need it
after all: reconciliation runs inline, inside the same connection handler
that just received the report, using the same outbound channel it already
has open. One code path, no shared registry, no extra concurrency to
reason about.

**Consequence — an accepted latency trade-off:** the agent sends a full
state report after `Welcome` (so reconnecting converges promptly), after
executing any command (so results are reflected quickly), and otherwise on
the same cadence as heartbeats (10s by default). If desired state changes
via the API while the agent is mid-interval, convergence can lag by up to
one heartbeat interval. Nothing in the roadmap promises low-latency
scheduling — this is judged an acceptable v1 trade-off, not an oversight.
If it stops being acceptable, the fix is a connection registry enabling an
immediate push, layered on top of this without changing the diff logic
itself.

## Storage

`observed_containers` is replaced wholesale per agent on every report
(`Store::replace_observed_containers`: delete-then-insert in one
transaction), matching the wire format — `ContainerStateReport` is a full
snapshot, not a delta, so persistence mirrors that instead of merging
incrementally. `desired_containers` is a plain upsert keyed on
`(agent_id, name)`; `DELETE /agents/{id}/containers/{name}` never deletes
the row, it flips `desired_status` to `absent` — the reconciler needs that
row to know a removal is still pending until the agent confirms it.

## Two things the smoke test caught that code review wouldn't have

**bollard doesn't pull images.** `docker run` auto-pulls a missing image;
bollard's `create_container` does not — it 404s with "No such image".
Found this by actually deploying `hello-world:latest` against a real
daemon that didn't have it cached. Fix: `ContainerManager::deploy`
(`crates/agent/src/container.rs`) now calls `create_image` (the pull API)
before `create_container`. Pull errors are deliberately swallowed rather
than propagated — the image might already exist locally under that exact
tag (offline dev, a custom-built image never pushed anywhere), in which
case `create_container` still succeeds; if the image genuinely doesn't
exist either way, `create_container`'s own error is the one that
surfaces.

**Report-after-every-command plus reconcile-on-every-report is a hot loop
when the command keeps failing.** With the bad-image bug above still in
place, the very first live test spent under 200ms sending ~17 rapid-fire
deploy attempts at the Docker daemon: fail → report (nothing changed,
still not converged) → server immediately re-sends the same `Deploy` →
fail again, no delay anywhere in the cycle. Fixed in
`crates/agent/src/stream.rs`: the immediate post-command state report
(added for fast success visibility) is now sent only when the command
*succeeded*. A failure is instead picked up by the next periodic report,
bounding retries to the heartbeat cadence — see "accepted limitation"
below.

Also added while fixing the above: `ContainerManager` now tracks the last
deploy error per container name (cleared on the next successful deploy or
on remove) and synthesizes a `ContainerStatus::Error` entry for it in
`list_state()`. Without this, a container that failed to even get created
had *nothing* to report — `list_containers` has no record of it — so it
looked identical to "nothing was ever attempted" instead of surfacing the
failure. Verified live: `GET /agents/{id}/containers` correctly showed
`{"name":"broken","status":"error"}` for a deliberately bad image
reference, at a stable ~3s retry cadence with no acceleration over a
20-second observation window.

**Accepted limitation, not yet fixed:** a persistently broken desired
container (bad image, wrong credentials, etc.) retries forever at the
heartbeat cadence with no backoff — confirmed live, retries stayed at a
steady ~3s rate rather than escalating, which is safe but will keep
hammering a registry that's genuinely down. A real orchestrator would add
something like Kubernetes' `ImagePullBackOff` here. Out of scope for
Phase 3; worth revisiting if it becomes a real problem (tracked as an open
question in `HARBORY_README.md` §8).

## Not done in Phase 3 (intentionally)

- **No connection registry / instant push**, as above.
- **No `Stop` in the automatic reconciler** — only reachable via a
  hypothetical future manual trigger, since "stopped" isn't a desired
  state in this model.
- **No restart policy, scaling, or scheduling** — explicitly out of scope
  per §1, not a gap to fill later in this phase.
- **No volumes or networks** — `ContainerSpec` covers image, env, ports,
  and command only. Add if/when a real use case needs them.
