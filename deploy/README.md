# Deployment

Installer scripts, one set per binary. All are safe to re-run and print
what they're doing at every step.

```bash
# On a control-plane host:
curl -fsSL https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-control-plane.sh | bash
# then edit /etc/harbory/control-plane.env and: sudo systemctl start harbory-control-plane

# On a host you want to manage as an agent (needs Docker; nginx optional):
curl -fsSL https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-agent.sh | bash
# then, once you have a pairing token from the dashboard:
sudo harbory-agent-pair <pairing-token>
```

Installing and pairing an agent are deliberately two separate steps.
`install-agent.sh` does everything that doesn't need a token yet — build,
system user, Docker group, nginx permissions, systemd unit — and installs
a `harbory-agent-pair` helper. Pairing tokens are short-lived, so you
often don't have one on hand until you're at the dashboard; and separating
the steps means re-pairing (agent got revoked, moving to a new account)
never requires touching system permissions again — just
`sudo harbory-agent-pair --force <new-token>`.

None of these scripts need to be run as root themselves — each builds the
binary as your own user via `cargo install`, then uses `sudo` only for the
specific system-level steps below. You'll get sudo password prompts as
needed.

## Why the agent needs sudo at all

`harbory-agent` runs as a dedicated, unprivileged `harbory-agent` system
user — not root, and not whichever user happened to run `cargo install`.
That user needs two things a normal account doesn't have:

1. **Docker socket access.** `install-agent.sh` adds `harbory-agent` to
   the `docker` group. (Same caveat as any Docker-group membership:
   equivalent to root on the host, via the daemon. This is the standard
   trust model Docker itself uses — not something specific to Harbory.)
2. **Writing `/etc/nginx/conf.d/harbory.conf` and reloading nginx**, for
   the reverse-proxy-management feature (`crates/agent/src/proxy.rs`).
   Rather than run the whole agent as root for this:
   - The installer pre-creates that *one* config file and `chown`s it to
     `harbory-agent`, so the agent can write it directly — no elevated
     access to the rest of `/etc/nginx`.
   - Reloading nginx means signalling its root-owned master process,
     which does require a privilege boundary crossing. The installer
     grants exactly that, and nothing more, via a sudoers rule scoped to
     two literal commands (`nginx -t` and `nginx -s reload`) and a tiny
     wrapper script (`/usr/local/bin/harbory-nginx-ctl`) that the agent
     invokes instead of `nginx` directly (`NGINX_BINARY_PATH` in
     `/etc/harbory/agent.env`).

If nginx isn't installed on the host, the installer skips this step —
the agent only touches nginx when a `ProxyConfig` command actually
arrives (see `docs/proxy-management.md`), so a container-only host
doesn't need it.

The control plane has no comparable privilege need — it only binds
`127.0.0.1:8080`/`:50051` (unprivileged ports; a real reverse proxy in
front of it, like the nginx+certbot setup on the hosted instance, is a
separate, manually-managed concern outside this script) and writes its
signing key into its own data directory. It still gets a dedicated
`harbory-control-plane` system user and a proper data directory, mainly
so a stray relative path doesn't put the signing key somewhere unexpected.

## What gets created

| | Control plane | Agent |
|---|---|---|
| binary | `/usr/local/bin/harbory-control-plane` | `/usr/local/bin/harbory-agent` (+ `harbory-agent-pair` helper) |
| system user | `harbory-control-plane` | `harbory-agent` (+ `docker` group) |
| data dir | `/var/lib/harbory-control-plane` (signing key) | `/var/lib/harbory-agent` (identity key, credential) |
| env file | `/etc/harbory/control-plane.env` | `/etc/harbory/agent.env` |
| systemd unit | `harbory-control-plane.service` | `harbory-agent.service` |

## Re-running / redeploying

Both scripts are idempotent: re-running rebuilds and reinstalls the
binary, leaves an existing env file alone, and restarts the service. This
is the redeploy path after pushing new commits — no separate "update"
script.

`harbory-agent-pair` skips pairing if a credential is already stored (an
already-paired agent keeps working after any redeploy — just re-run
`install-agent.sh`, no need to re-pair). Pass `--force` to wipe the stored
credential and re-pair with a fresh token — useful if an agent was
revoked and needs to rejoin under a new pairing token.

## Applying this to an already-deployed control plane

`install-control-plane.sh` refuses to touch an existing
`harbory-control-plane.service` unless you pass `--force` — a hand-set-up
instance (like the current hosted one, running as whatever user set it up
originally with `~/harbory.env`) has a working `DATABASE_URL` and
Supabase config that a fresh installer run can't know about. If you do
want to migrate it onto this script's user/directory layout, copy the
values out of the existing env file into `/etc/harbory/control-plane.env`
first, *then* re-run with `--force`.
