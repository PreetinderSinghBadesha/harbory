# apt repository for harbory-agent

Packages `harbory-agent` as a `.deb` and serves it from a self-hosted apt
repository at `harbory-apt.preetindersingh.tech/apt/`. Every third-party
apt repo (Docker's, Google Chrome's, this one) needs a one-time bootstrap
before `apt install` can see it — apt refuses to trust a source it
hasn't been told about, by design. `add-apt-repo.sh` collapses that into
one command:

```bash
curl -fsSL https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/add-apt-repo.sh | sudo bash
sudo apt install harbory-agent
```

(equivalent to, and safe to run instead of, the key-add + source-add +
`apt update` sequence by hand — see the script for exactly what it does.)

This is an *alternative* install path to `deploy/install-agent.sh`'s
`curl | bash`, not a replacement — both converge on the exact same
end-state (same service user, same nginx wrapper/sudoers, same env file
layout). The tradeoff: `install-agent.sh` builds from source and needs
re-running to update; the apt path needs this one-time bootstrap but then
`apt upgrade harbory-agent` works forever after. Pick whichever fits your
fleet. See "What the package does" below.

## 1. Build and verify the `.deb` locally (on a real Linux box — the
   tools here don't exist on this project's Windows dev machine)

```bash
cargo install cargo-deb --locked
cargo build --release -p harbory-agent
cargo deb -p harbory-agent --no-build
# -> target/debian/harbory-agent_<version>_<arch>.deb

# Inspect before installing:
dpkg-deb --info target/debian/harbory-agent_*.deb
dpkg-deb --contents target/debian/harbory-agent_*.deb

# Real install/uninstall smoke test, ideally in a disposable VM/container:
sudo dpkg -i target/debian/harbory-agent_*.deb
systemctl status harbory-agent.service   # inactive, not-started — expected
id harbory-agent                         # service user exists
cat /etc/harbory/agent.env               # env file was written
sudo harbory-agent-pair <pairing-token>  # now actually pair it
sudo apt remove harbory-agent            # binary/unit gone, config kept
sudo apt purge harbory-agent             # config gone too; data dir/user kept — see postrm's comment
```

`[package.metadata.deb]` in `crates/agent/Cargo.toml` defines the package
metadata and file layout; `deploy/debian/{postinst,prerm,postrm}` replicate
`install-agent.sh`'s setup logic (service user, docker group, nginx
wrapper + scoped sudoers rule, env file) as Debian maintainer scripts —
read `install-agent.sh`'s own comments first if you're changing either,
since they're meant to stay in lockstep.

### What the package does — and deliberately doesn't do

- Installs the binary, a `harbory-agent-pair` helper, and the systemd
  unit; creates the `harbory-agent` system user and its data directory;
  writes `/etc/harbory/agent.env`; sets up the nginx reverse-proxy
  permission chain (wrapper script + scoped `sudoers.d` rule) if nginx is
  present. All idempotent, same as the shell installer.
- Does **not** start or enable the service — pairing needs a token from
  the dashboard you may not have yet. Pair (and re-pair) with
  `sudo harbory-agent-pair <pairing-token>`, same as the shell-installed
  version.
- Does **not** hard-`Depends` on a Docker package — Docker's package name
  varies too much across install methods (`docker.io`, `docker-ce` from
  Docker's own repo, snap, a manual binary) to safely express as a single
  apt dependency. It's a `Recommends`, and the agent itself already fails
  fast with a clear error if the daemon isn't reachable.
- `apt remove` stops the service and removes the binary/unit but leaves
  `/etc/harbory/agent.env` (it's declared a `conf-file`, apt's normal
  behavior). `apt purge` removes that plus the generated nginx/sudoers
  files — but **not** `/var/lib/harbory-agent` or the `harbory-agent`
  user, since that directory holds the agent's private key and
  control-plane credential; auto-deleting it on purge could silently
  orphan a live paired agent. Remove it by hand if that's genuinely what
  you want (`postrm`'s comment has the exact command).

## 2. Set up the repository (reprepro)

`apt-repo/conf/distributions` and `apt-repo/conf/options` are checked into
this repo — they're the *definition* of the repo, not its generated
output. `dists/`, `pool/`, and reprepro's `db/`/`lists/` state directories
are gitignored and only ever exist on whatever host actually serves the
repo (or transiently in CI before syncing).

```bash
sudo apt install reprepro gnupg

cd apt-repo
reprepro includedeb stable /path/to/harbory-agent_<version>_amd64.deb
reprepro includedeb stable /path/to/harbory-agent_<version>_arm64.deb
# -> populates apt-repo/dists/ and apt-repo/pool/
```

## 3. Generate the signing key

A **dedicated** key for this repo, not a personal one — and
deliberately **passphrase-less**, since `reprepro` runs unattended in CI
with nothing to type a passphrase into. This is the standard tradeoff for
an automated package repo (Launchpad PPAs and most distro CI signing keys
work the same way): the key's confidentiality rests entirely on GitHub
Secrets and the server's file permissions, not on a passphrase. If that
tradeoff doesn't sit right for your threat model, sign manually on your
own machine instead of via the GitHub Actions workflow.

```bash
gpg --batch --pinentry-mode loopback --passphrase '' --quick-generate-key \
  "Harbory apt repo <preetindersingh13per@gmail.com>" ed25519 sign never

# Find its fingerprint:
gpg --list-secret-keys --keyid-format long

# Point reprepro at it explicitly (recommended over conf/distributions'
# default `SignWith: yes`, which just picks "the only secret key" and
# breaks silently if your gnupg homedir ever has more than one):
#   SignWith: <fingerprint>

# Export the public key for distribution — this is what end users import:
gpg --armor --export "Harbory apt repo" > apt-repo/harbory-archive-keyring.asc
```

Keep the private key **out of git** — export it once for the GitHub
Actions secret below, then treat your local copy as sensitive:

```bash
gpg --export-secret-keys --armor "Harbory apt repo" | base64 -w0
# -> paste as the APT_GPG_PRIVATE_KEY repo secret
```

## 4. Host it

Serve `apt-repo/dists/`, `apt-repo/pool/`, and
`apt-repo/harbory-archive-keyring.asc` as static files under
`harbory-apt.preetindersingh.tech/apt/` — a dedicated subdomain, not the
main frontend's domain, so it needs its own DNS record and Certbot cert
first (`sudo certbot --nginx -d harbory-apt.preetindersingh.tech`):

```nginx
server {
    listen 443 ssl http2;
    server_name harbory-apt.preetindersingh.tech;

    location /apt/ {
        alias /var/www/harbory/apt/;
        autoindex off;
    }
}
```

## 4b. The restricted deploy key

CI needs to write into the apt directory and nothing else, so it gets its
own key rather than the server's admin key:

```bash
# On the server
sudo useradd -m -s /bin/bash harbory-apt-deploy
sudo mkdir -p /var/www/harbory/apt
sudo chown -R harbory-apt-deploy:harbory-apt-deploy /var/www/harbory/apt

sudo tee /usr/local/bin/harbory-apt-rsync >/dev/null <<'EOF'
#!/bin/sh
case "$SSH_ORIGINAL_COMMAND" in
  "rsync --server "*) exec $SSH_ORIGINAL_COMMAND ;;
  *) echo "rejected: this key may only run rsync" >&2; exit 1 ;;
esac
EOF
sudo chmod 755 /usr/local/bin/harbory-apt-rsync
```

Then in `/home/harbory-apt-deploy/.ssh/authorized_keys`, prefix the
public key with:

```
command="/usr/local/bin/harbory-apt-rsync",no-pty,no-agent-forwarding,no-X11-forwarding,no-port-forwarding ssh-ed25519 AAAA...
```

Two independent restrictions, deliberately: the `command=` wrapper means
the key can only ever run rsync — no shell, no arbitrary commands — and
filesystem ownership means rsync can only write into the apt directory
whatever paths it is handed.

**Why not `rrsync`?** It's the conventional choice and it does enforce
path containment itself, but it is tightly coupled to the rsync version
it shipped with: an rrsync from a newer branch passes options (e.g.
`--drop-D`) that an older server-side rsync rejects outright, and the
error surfaces as an opaque protocol failure. With three rsync versions
in play (CI runner, server, and any local machine testing a deploy),
keeping them aligned is ongoing work for a guarantee filesystem
ownership already provides here.

**Store the private key base64-encoded** in the GitHub secret. A raw PEM
pasted through the web UI picks up CRLF line endings or loses its
trailing newline, and OpenSSH then rejects it with `error in libcrypto`:

```bash
base64 -w0 ~/harbory-apt-deploy-key   # -> APT_DEPLOY_SSH_KEY
```

## 5. Automate via GitHub Actions

`.github/workflows/release-deb.yml` builds both architectures on a
pushed `vX.Y.Z` tag, `reprepro includedeb`s them, exports the public key,
and `rsync`s the result to the server over SSH. These are **environment
secrets** scoped to a `harbory` environment (Settings → Environments),
not plain repo secrets — the `publish` job must declare
`environment: harbory` or they silently resolve to empty strings instead
of failing loudly:

| Secret | What |
|---|---|
| `APT_GPG_PRIVATE_KEY` | `base64 -w0` of the exported private key (step 3) |
| `APT_DEPLOY_SSH_KEY` | `base64 -w0` of the restricted deploy key (step 4b) — a raw pasted PEM picks up CRLF/newline damage and OpenSSH rejects it with `error in libcrypto` |
| `APT_DEPLOY_HOST` | e.g. `harbory-apt.preetindersingh.tech` |
| `APT_DEPLOY_USER` | `harbory-apt-deploy` |
| `APT_DEPLOY_PATH` | Absolute path served as `/apt/` (e.g. `/var/www/harbory/apt`) |

Cutting a release is then just: `git tag v0.2.0 && git push origin v0.2.0`
(or re-run via `workflow_dispatch` from the Actions tab, which doesn't
need a tag move — useful while iterating on the workflow itself).

## 6. Installing on a target VM

```bash
curl -fsSL https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/add-apt-repo.sh | sudo bash
sudo apt install harbory-agent
sudo harbory-agent-pair <pairing-token>
```

Or by hand, equivalent to what that script does:

```bash
curl -fsSL https://harbory-apt.preetindersingh.tech/apt/harbory-archive-keyring.asc \
  | sudo gpg --dearmor -o /usr/share/keyrings/harbory-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/harbory-archive-keyring.gpg] \
  https://harbory-apt.preetindersingh.tech/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/harbory.list
sudo apt update
sudo apt install harbory-agent
sudo harbory-agent-pair <pairing-token>
```

Upgrades are then just `sudo apt update && sudo apt upgrade harbory-agent`
— no re-pairing needed, same as restarting the shell-installed binary.
