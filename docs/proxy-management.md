# Proxy management (Phase 4)

## Model

One desired `ProxyRoute` = one Nginx `server{}` block (name-based virtual
hosting). v1 deliberately does not merge multiple routes that share a
`server_name`+`listen_port` into a single block with several `location`s —
that grouping logic is real complexity with its own edge cases (which
route "wins" for the same host+port+prefix), and the common case this
project targets — one container, one subdomain — doesn't need it.
Consequence for operators: keep `server_name` distinct per route; nginx
will accept multiple `server{}` blocks with the same `server_name` +
`listen_port`, but will only ever match requests against the first one it
finds, silently ignoring the others.

Unlike containers, a proxy route has no separate "absent" status —
presence of a `desired_proxy_routes` row *is* "this route should exist."
`DELETE /agents/{id}/proxy-routes/{name}` deletes the row outright. This
is possible (and simpler than the container model) specifically because
proxy config is regenerated as one whole file from the complete route set
on every apply, not patched incrementally — there's no equivalent of "tell
Docker to remove exactly this one container," so there's nothing that
needs a distinct "please remove this" signal separate from "it's no longer
in the desired set."

## Reconciliation trigger — same pattern as containers, reused deliberately

The agent tracks a hash of whatever route set it last successfully
applied and reports it (`ProxyState.applied_hash`) on the same cadence as
heartbeats and container state — after `Welcome`, after any apply attempt
that succeeds, and periodically otherwise. The control plane computes the
hash of current desired state and, if it doesn't match what the agent
reported, sends the full route set back as `ProxyConfig`
(`reconcile_proxy_and_dispatch` in `crates/control-plane/src/stream.rs`,
directly parallel to `reconcile_and_dispatch` for containers). No
connection registry, no instant push on `PUT`/`DELETE` — same accepted
up-to-one-heartbeat-interval latency trade-off as `docs/reconciliation.md`
describes for containers, chosen again here specifically to keep reusing
one architectural pattern rather than inventing a proxy-specific one.

**The hash must be computed identically on both sides**, or the control
plane would either never stop re-sending (agent's hash never "matches")
or falsely believe convergence happened. To make a mismatch structurally
impossible rather than merely unlikely, the hash function
(`crates/protocol/src/proxy_hash.rs::hash_routes`) lives in the shared
`harbory-protocol` crate — both sides call the literal same code, not two
independent reimplementations. It sorts routes by name first (order
shouldn't matter) and length-prefixes every field before hashing (so
`name="a", server_name="bc"` can never collide with `name="ab",
server_name="c"`).

On a fresh connection the agent hasn't applied anything yet, so it reports
an empty `applied_hash` — deliberately distinct from `hash_routes(&[])`
(a real 32-byte value), so the very first report always mismatches even
if desired state is genuinely empty. That first round trip establishes a
verified baseline rather than assuming "empty until proven otherwise."

Same anti-hot-loop rule as containers, for the same reason (see
`docs/reconciliation.md`'s writeup of the bug found there): the agent only
sends an immediate report after a *successful* apply. A failed apply is
picked up at the next periodic report instead, so a persistently-broken
route (bad upstream, whatever) retries at the heartbeat cadence rather
than hammering `nginx -t` in a tight loop.

## Validate before applying, graceful reload, rollback

`ProxyManager::apply` (`crates/agent/src/proxy.rs`):

1. Read and hold the current contents of the managed config file in
   memory (`None` if it doesn't exist yet).
2. Write the newly rendered config to that same real path.
3. Run `nginx -t`. This deliberately tests the file **in place at its real
   path**, not a shadow copy — `nginx -t` validates the *effective*
   config (main `nginx.conf` plus everything it `include`s), so a shadow
   copy sitting outside that include chain wouldn't actually get
   validated at all. The real path is safe to write to before validation
   passes because of the next point.
4. **If validation fails:** restore the file to what it held before (or
   delete it, if there was nothing before), and return the error —
   without ever calling `-s reload`. The nginx *master process* still has
   the old config loaded in memory; it was never told to switch over, so
   there is no live-traffic impact from the brief window where the
   on-disk file held content that failed validation.
5. **If validation succeeds:** `nginx -s reload` — graceful (existing
   connections drain, new workers start with the new config), never a
   restart.

**Rollback, therefore, mostly isn't a separate mechanism** — it falls out
of two things already true: (a) the write-then-restore-on-failure
sequence above never leaves an invalid file behind, and (b) nginx's own
`-s reload` semantics refuse to tear down working old workers in favor of
a config that never even got tested. The one gap this doesn't cover: a
config that passes `nginx -t` but somehow misbehaves only at reload/runtime
(rare — e.g. a port already bound by something else). Out of scope for
v1; would show up as a `Reload` error surfaced via `ProxyState.error`,
with the *previous* config still serving traffic either way, since reload
failure doesn't tear down the running workers either.

## Concurrency: "should serialize, not clobber"

`ProxyManager` holds a `tokio::sync::Mutex<()>` around the entire
read-write-test-reload sequence in `apply`. In practice, `ProxyConfig`
commands only ever arrive one at a time — they come from the single
per-connection message loop in `crates/agent/src/stream.rs`, which
processes one inbound message before reading the next — so nothing today
can actually call `apply` concurrently. The lock exists anyway to make
that a structural guarantee rather than an accident of the current call
graph, since the roadmap explicitly calls out handling this race as a
requirement, not an implementation detail to skip because it happens not
to trigger yet.

## Not tested live in this phase, and why

Every other phase's smoke test ran the real compiled `harbory-agent`
binary against real infrastructure (real Postgres, real Docker). This
phase's dev environment is Windows with no nginx installed natively, so
that wasn't possible for the full apply/validate/reload/rollback path
end-to-end through the actual running agent.

What *was* verified against real nginx: the exact output of
`proxy::render` (via `cargo run -p harbory-agent --example
render_proxy_config`, piped into a scratch `nginx:alpine` container's
`/etc/nginx/conf.d/`) passes `nginx -t` — `nginx: configuration file
/etc/nginx/nginx.conf test is successful`. That confirms the template
itself produces syntactically valid config, which is the part most likely
to have a real bug (as Phase 3's bollard image-pull surprise showed,
"the abstraction should just work" is not something to assume without
checking). The `apply` function's file-backup/restore and subprocess
sequencing were reviewed carefully but not exercised against a live nginx
process. Revisit this the first time Phase 5 or later work runs on an
actual Linux host with nginx installed — that's the point to add a real
end-to-end smoke test.

## Not done in Phase 4 (intentionally)

- **No merging of same-host routes into one `server{}` block** — see
  "Model" above.
- **No TLS termination** — HTTP only. Consistent with the project's other
  deferred-TLS decisions (`docs/security.md`); real cert provisioning
  (ACME/Let's Encrypt or otherwise) is a substantial feature of its own,
  not a natural fit for the phase that also had to build the basic
  validate/reload plumbing.
- **No load balancing / multiple upstreams per route** — one
  `upstream_host`/`upstream_port` pair per route.
