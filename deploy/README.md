# Deployment

Installer scripts, one set per binary. All are safe to re-run and print
what they're doing at every step. `harbory-agent` can also be installed
via `apt` from a self-hosted repository instead of `curl | bash` — see
[apt-repo.md](apt-repo.md).

```bash
# On a control-plane host:
curl -fsSL https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-control-plane.sh | bash
# then edit /etc/harbory/control-plane.env and: sudo systemctl start harbory-control-plane

# On a host you want to manage as an agent (Docker is a hard requirement;
# nginx and git are installed automatically via apt/yum if missing):
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

The installer installs nginx (and `git`, needed for git-sourced
container deploys — see "Deploying from a GitHub repo" below) via
`apt-get`/`yum` if either is missing, so this setup normally always
runs. It only skips if neither package manager is available to install
them with, or if the generated sudoers rule fails validation — either
way you get a clear message rather than a silent gap, since the agent
only touches nginx when a `ProxyConfig` command actually arrives (see
`docs/proxy-management.md`), so a container-only host can still work
without it.

Right after the sudoers rule is installed, the script also **runs the
whole chain end to end** — `sudo -u harbory-agent
harbory-nginx-ctl -t` — and prints a clear OK/FAILED instead of letting
a broken link only surface later as an opaque "Last apply error" on the
dashboard with no access to that host's logs to debug it from. If you
ever see FAILED here, proxy-route deploys won't work until it's fixed;
the output tells you which link in the chain broke.

Separately, the systemd unit itself has an `ExecStartPre` that runs
`docker info` as the same `harbory-agent` user before the real process
starts — if Docker ever breaks after install (uninstalled, daemon down),
`systemctl status harbory-agent` shows Docker's own error directly
instead of a Rust panic buried in the journal. This only helps local
diagnosis, though: a broken Docker still just shows as "offline" on the
dashboard, since the agent can't reach the control plane to report
anything more specific.

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

## Deploying from a GitHub repo

Beyond a plain image, a container can be built and deployed straight
from a git repo (public or private) — the agent clones and `docker
build`s it locally, no registry involved. This needs the control plane
to have a GitHub OAuth App connected, which is entirely optional
config, not something `install-control-plane.sh` sets up for you:

1. Register an OAuth App on GitHub (Settings → Developer settings →
   OAuth Apps → New OAuth App). Set **Authorization callback URL** to
   `{your control-plane HTTP domain}/github/oauth/callback`. Uncheck
   **"Expire user access tokens"** — leaving it checked issues a
   short-lived token plus a refresh token, and the control plane
   doesn't implement the refresh flow (yet), so a connection would
   silently stop working after ~8 hours.
2. Add four env vars to `/etc/harbory/control-plane.env`:
   ```
   GITHUB_CLIENT_ID=<from the OAuth App>
   GITHUB_CLIENT_SECRET=<from the OAuth App>
   GITHUB_REDIRECT_URI=https://your-control-plane-domain/github/oauth/callback
   FRONTEND_URL=https://your-dashboard-domain
   ```
   `GITHUB_REDIRECT_URI` must match the callback URL registered in step
   1 exactly. `FRONTEND_URL` is where the OAuth flow redirects the
   browser back to once it completes (the Settings page).
3. `sudo systemctl restart harbory-control-plane`.

Without these set, the `/github/*` routes just return 503 — the control
plane still starts and runs fine, this is additive, not required.
Nothing agent-side needs configuring for this: `install-agent.sh`
already ensures `git` is present (see "Why the agent needs sudo at all"
above), and the credential for a private repo is embedded into the
clone URL only in the message the control plane sends to the agent at
deploy time — never written to `desired_containers` or any file on
disk beyond that one ephemeral clone.

## Applying this to an already-deployed control plane

`install-control-plane.sh` refuses to touch an existing
`harbory-control-plane.service` unless you pass `--force` — a hand-set-up
instance (like the current hosted one, running as whatever user set it up
originally with `~/harbory.env`) has a working `DATABASE_URL` and
Supabase config that a fresh installer run can't know about. If you do
want to migrate it onto this script's user/directory layout, copy the
values out of the existing env file into `/etc/harbory/control-plane.env`
first, *then* re-run with `--force`.
