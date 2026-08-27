import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { CopyButton } from "../components/CopyButton";
import { GitHubMark } from "../components/GitHubMark";
import { misuseIcon, spriteFor } from "../lib/agentSprite";
import greenDrive1 from "../assets/sprites/robot_greenDrive1.png";
import greenDrive2 from "../assets/sprites/robot_greenDrive2.png";
import greenHurt from "../assets/sprites/robot_greenHurt.png";
import blueDrive1 from "../assets/sprites/robot_blueDrive1.png";
import yellowDrive1 from "../assets/sprites/robot_yellowDrive1.png";
import redDrive1 from "../assets/sprites/robot_redDrive1.png";
import "../styles/GameHud.css";
import "./Docs.css";

/** Not a real agent id — just a stable input so the guide character gets
 * one deterministic color (same convention as agent sprites: hashed, not
 * random or configurable) rather than picking arbitrarily. */
const GUIDE_SPRITE = spriteFor("harbory-docs-guide");

const GITHUB_URL = "https://github.com/PreetinderSinghBadesha/harbory";
const DOCS_FOLDER_URL = "https://github.com/PreetinderSinghBadesha/harbory/tree/master/docs";
const INSTALL_SCRIPT_URL = "https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-agent.sh";
const ADD_APT_REPO_SCRIPT_URL = "https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/add-apt-repo.sh";
const CONTROL_PLANE_SCRIPT_URL = "https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-control-plane.sh";
const SOURCE_INSTALL_COMMAND = `curl -fsSL ${INSTALL_SCRIPT_URL} | bash`;
const ADD_APT_REPO_COMMAND = `curl -fsSL ${ADD_APT_REPO_SCRIPT_URL} | sudo bash`;
const APT_INSTALL_COMMAND = "sudo apt install harbory-agent";
const PAIR_COMMAND = "sudo harbory-agent-pair <pairing-token>";
const REPAIR_COMMAND = "sudo harbory-agent-pair --force <new-pairing-token>";
const CONTROL_PLANE_COMMAND = `curl -fsSL ${CONTROL_PLANE_SCRIPT_URL} | bash`;

const TOC = [
  { id: "prerequisites", label: "Prerequisites" },
  { id: "install", label: "1. Install" },
  { id: "pair", label: "2. Pair" },
  { id: "permissions", label: "Why sudo is needed" },
  { id: "created", label: "What gets created" },
  { id: "control-plane", label: "Running a control plane" },
  { id: "github", label: "Deploying from a repo" },
  { id: "redeploy", label: "Redeploying & re-pairing" },
  { id: "troubleshooting", label: "Troubleshooting" },
  { id: "sprites", label: "The robots" },
];

function SpriteCard({
  stageClass,
  children,
  badge,
  badgeStyle,
  caption,
}: {
  stageClass: string;
  children: React.ReactNode;
  badge: string;
  badgeStyle: React.CSSProperties;
  caption: string;
}) {
  return (
    <div className="pixel-panel-sm" style={{ padding: "18px 14px 16px", textAlign: "center", background: "var(--bg)" }}>
      <div className="sprite-sky">
        <div className={`sprite-stage ${stageClass}`}>{children}</div>
        <div className="ground" />
      </div>
      <div style={{ marginTop: 10, marginBottom: 8 }}>
        <span className="badge" style={badgeStyle}>{badge}</span>
      </div>
      <p className="mono" style={{ fontSize: 11.5, lineHeight: 1.6, color: "var(--muted)", margin: 0 }}>
        {caption}
      </p>
    </div>
  );
}

function CodeBlock({ command, label, step }: { command: string; label: string; step?: number }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
      {step !== undefined && (
        <span className="pixel" style={{ color: "var(--clay-dark)", fontSize: 9, flexShrink: 0 }}>{step}</span>
      )}
      <div className="pixel-panel-sm" style={{ display: "flex", alignItems: "center", gap: 10, padding: "12px 14px", background: "#FFF9EE", flex: 1, minWidth: 0 }}>
        <span className="mono" style={{ color: "var(--clay-dark)", fontSize: 13, fontWeight: 700, flexShrink: 0 }}>$</span>
        <code className="mono" style={{ flex: 1, minWidth: 0, fontSize: 12, overflowX: "auto", whiteSpace: "nowrap" }}>{command}</code>
        <CopyButton text={command} label={label} />
      </div>
    </div>
  );
}

function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }) {
  return (
    <section id={id} style={{ marginBottom: 56, scrollMarginTop: 100 }}>
      <h2 style={{ fontFamily: "var(--font-sans)", fontSize: 22, fontWeight: 800, letterSpacing: "-0.01em", margin: "0 0 16px" }}>
        {title}
      </h2>
      {children}
    </section>
  );
}

function jumpTo(e: React.MouseEvent, id: string) {
  e.preventDefault();
  document.getElementById(id)?.scrollIntoView({ behavior: "smooth" });
  history.pushState(null, "", `#${id}`);
}

/** The "ON THIS PAGE" nav — tracks scroll position via IntersectionObserver
 * rather than a static list, so pips fill in as you read past each section
 * and the current one stays highlighted. Desktop-only (see .docs-trail /
 * .docs-trail-mobile in Docs.css); a plain horizontal link row covers
 * narrow viewports where there's no margin to float a side panel in. */
function DocsTrail() {
  const [activeId, setActiveId] = useState(TOC[0].id);

  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) setActiveId(entry.target.id);
        }
      },
      { rootMargin: "-110px 0px -70% 0px", threshold: 0 },
    );
    TOC.forEach((item) => {
      const el = document.getElementById(item.id);
      if (el) observer.observe(el);
    });
    return () => observer.disconnect();
  }, []);

  const activeIndex = TOC.findIndex((t) => t.id === activeId);

  return (
    <>
      <nav className="docs-trail pixel-panel-sm" aria-label="On this page">
        <div className="pixel" style={{ fontSize: 9.5, color: "var(--muted)", marginBottom: 20 }}>ON THIS PAGE</div>
        <div className="docs-trail-line" />
        <div className="docs-trail-list">
          {TOC.map((item, i) => (
            <a
              key={item.id}
              href={`#${item.id}`}
              onClick={(e) => jumpTo(e, item.id)}
              className={`docs-trail-item${i === activeIndex ? " docs-trail-item-active" : ""}${i < activeIndex ? " docs-trail-item-done" : ""}`}
            >
              <span className="docs-trail-pip" />
              <span className="docs-trail-label">{item.label}</span>
            </a>
          ))}
        </div>
      </nav>

      <nav className="docs-trail-mobile" aria-label="On this page">
        {TOC.map((item, i) => (
          <a
            key={item.id}
            href={`#${item.id}`}
            onClick={(e) => jumpTo(e, item.id)}
            className={`docs-toc-link${i === activeIndex ? " docs-toc-link-active" : ""}`}
          >
            {item.label}
          </a>
        ))}
      </nav>
    </>
  );
}

export function Docs() {
  return (
    <div className="game-hud">
      <header style={{ background: "var(--panel)", borderBottom: "4px solid var(--ink)", position: "sticky", top: 0, zIndex: 20 }}>
        <div className="docs-section-inner" style={{ height: 94, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <Link to="/" style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div className="pixel-panel-sm" style={{ width: 32, height: 32, background: "var(--clay)", display: "flex", alignItems: "center", justifyContent: "center" }}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round">
                <path d="M12 2.5 L21 7.5 V16.5 L12 21.5 L3 16.5 V7.5 Z" />
                <path d="M3 7.5 L12 12.5 L21 7.5" />
                <path d="M12 12.5 V21.5" />
              </svg>
            </div>
            <span className="pixel" style={{ fontSize: 14 }}>HARBORY</span>
          </Link>
          <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
            <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer" className="pixel-btn pixel-btn-ghost pixel-btn-sm">
              <GitHubMark />
              <span className="docs-gh-label">GITHUB</span>
            </a>
            <Link to="/dashboard" className="pixel-btn pixel-btn-sm">DASHBOARD</Link>
          </div>
        </div>
      </header>

      <div className="docs-section-inner" style={{ padding: "56px 32px 40px" }}>
        <div className="docs-intro">
          <div>
            <div className="eyebrow">DOCUMENTATION</div>
            <h1 style={{ fontFamily: "var(--font-sans)", fontSize: 36, fontWeight: 800, letterSpacing: "-0.02em", margin: "0 0 14px" }}>
              Install and pair an agent.
            </h1>
            <p style={{ fontSize: 15.5, lineHeight: 1.6, color: "var(--muted)", maxWidth: "62ch", margin: 0 }}>
              Two scripts, deliberately separate: an installer sets up everything a host needs before it can run an
              agent at all, and <span className="mono">harbory-agent-pair</span> connects it to a control plane
              using a short-lived pairing token from the dashboard. Both are safe to re-run. Install via{" "}
              <span className="mono">apt</span> (recommended — updates track <span className="mono">apt upgrade</span>{" "}
              afterward) or by building from source — both converge on the same end state.
            </p>
          </div>
          <div className="docs-intro-guide sprite-stage sprite-deco-bob" aria-hidden="true">
            <img className="f1" src={GUIDE_SPRITE.drive1} alt="" />
            <img className="f2" src={GUIDE_SPRITE.drive2} alt="" />
          </div>
        </div>
      </div>

      <div className="docs-section-inner" style={{ padding: "0 32px 96px" }}>
        <DocsTrail />
        <div className="docs-content">
            <Section id="prerequisites" title="Prerequisites">
              <div className="alert-row alert-row-info" style={{ marginTop: 0 }}>
                <strong>Docker is required.</strong> The agent connects to the local Docker socket at startup and
                exits immediately if it can't — container management is its core job, not an optional feature.
              </div>
              <div className="alert-row alert-row-warn">
                <strong>nginx and git are handled for you.</strong> The installer installs both automatically (via
                <span className="mono"> apt</span>/<span className="mono">yum</span>) if either is missing — nginx
                for proxy routes, git for deploying containers from a repo. Neither is strictly required: a
                container-only host works fine without nginx, and git only matters if you use the GitHub deploy
                path below.
              </div>
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)" }}>
                You'll also need <span className="mono">cargo</span> (install Rust via{" "}
                <a href="https://rustup.rs" target="_blank" rel="noopener noreferrer" style={{ color: "var(--clay-dark)", fontWeight: 600 }}>
                  rustup.rs
                </a>{" "}
                if it's missing) and passwordless-capable <span className="mono">sudo</span> access — the script
                builds as your own user, then uses <span className="mono">sudo</span> only for the specific
                system-level steps below.
              </p>
            </Section>

            <Section id="install" title="1. Install">
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: "0 0 16px" }}>
                Run one of these on the host you want to manage. No pairing token needed yet — neither path starts
                the agent, just gets the host ready.
              </p>

              <div className="field-label" style={{ marginBottom: 8 }}>OPTION A — APT (UBUNTU/DEBIAN, RECOMMENDED)</div>
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "0 0 14px" }}>
                A one-time repo bootstrap, then a normal install that hooks into{" "}
                <span className="mono">apt upgrade</span> for future updates:
              </p>
              <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                <CodeBlock command={ADD_APT_REPO_COMMAND} label="add repo command" step={1} />
                <CodeBlock command={APT_INSTALL_COMMAND} label="apt install command" step={2} />
              </div>
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "18px 0 0" }}>
                Step 1 only ever needs running once per host; it registers Harbory's signing key and apt source
                (every third-party repo — Docker's, Chrome's — needs the same one-time step, apt refuses to trust
                a source it hasn't been told about). Creates the same end state as Option B below: a dedicated,
                unprivileged <span className="mono">harbory-agent</span> system user, its own data directory, a
                systemd unit (installed, not started yet), and the{" "}
                <span className="mono">harbory-agent-pair</span> helper used in the next step. See{" "}
                <span className="mono">deploy/apt-repo.md</span> on GitHub for the full repo-hosting story.
              </p>

              <div className="field-label" style={{ margin: "28px 0 8px" }}>OPTION B — BUILD FROM SOURCE</div>
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "0 0 14px" }}>
                No repo, no apt — builds and installs directly. Update by re-running this script rather than{" "}
                <span className="mono">apt upgrade</span>:
              </p>
              <CodeBlock command={SOURCE_INSTALL_COMMAND} label="install command" />
            </Section>

            <Section id="pair" title="2. Pair">
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: "0 0 16px" }}>
                Generate a pairing token from the <Link to="/dashboard" style={{ color: "var(--clay-dark)", fontWeight: 600 }}>dashboard</Link> —
                it's single-use and expires a few minutes after issue — then run:
              </p>
              <CodeBlock command={PAIR_COMMAND} label="pair command" />
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "18px 0 0" }}>
                This connects to the control plane, stores the signed credential it gets back, then enables and
                starts <span className="mono">harbory-agent.service</span>. If a credential is already stored,
                pairing is skipped and the service just (re)starts — safe to run again after any redeploy.
              </p>
            </Section>

            <Section id="permissions" title="Why sudo is needed">
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: "0 0 20px" }}>
                <span className="mono">harbory-agent</span> runs as its own unprivileged system user, never root.
                That user gets exactly two elevated grants — nothing broader:
              </p>
              <table>
                <thead>
                  <tr>
                    <th>Grant</th>
                    <th>Scope</th>
                    <th>Why</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><span className="badge" style={{ background: "#E9F4FF", color: "var(--blue)", borderColor: "var(--blue)" }}>docker</span> group</td>
                    <td>Full Docker socket access</td>
                    <td>Container management is Docker's own trust model — group membership is effectively root via the daemon, same as any Docker install.</td>
                  </tr>
                  <tr>
                    <td>sudoers rule</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>nginx -t, nginx -s reload</td>
                    <td>Reloading nginx means signalling its root-owned master process — the narrowest possible grant that still works, via a tiny wrapper script.</td>
                  </tr>
                </tbody>
              </table>
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "16px 0 0" }}>
                Writing <span className="mono">/etc/nginx/conf.d/harbory.conf</span> itself needs no special
                privilege at all — the installer pre-creates that one file and{" "}
                <span className="mono">chown</span>s it to <span className="mono">harbory-agent</span>, so the
                agent can write it directly without access to the rest of <span className="mono">/etc/nginx</span>.
              </p>
            </Section>

            <Section id="created" title="What gets created">
              <table>
                <thead>
                  <tr>
                    <th></th>
                    <th>Control plane</th>
                    <th>Agent</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>binary</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>/usr/local/bin/harbory-control-plane</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>/usr/local/bin/harbory-agent</td>
                  </tr>
                  <tr>
                    <td>system user</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>harbory-control-plane</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>harbory-agent (+ docker group)</td>
                  </tr>
                  <tr>
                    <td>data dir</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>/var/lib/harbory-control-plane</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>/var/lib/harbory-agent</td>
                  </tr>
                  <tr>
                    <td>env file</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>/etc/harbory/control-plane.env</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>/etc/harbory/agent.env</td>
                  </tr>
                  <tr>
                    <td>systemd unit</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>harbory-control-plane.service</td>
                    <td className="mono" style={{ fontSize: 11.5 }}>harbory-agent.service</td>
                  </tr>
                </tbody>
              </table>
            </Section>

            <Section id="control-plane" title="Running a control plane">
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: "0 0 16px" }}>
                Spinning up your own control plane instead of pointing agents at a hosted one:
              </p>
              <CodeBlock command={CONTROL_PLANE_COMMAND} label="control plane install command" />
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "18px 0 0" }}>
                Writes placeholders to <span className="mono">/etc/harbory/control-plane.env</span> — fill in a
                real <span className="mono">DATABASE_URL</span> and Supabase config, then{" "}
                <span className="mono">sudo systemctl start harbory-control-plane</span>. It only binds{" "}
                <span className="mono">127.0.0.1</span>, so a reverse proxy with a real TLS cert in front of it is
                a separate, manual concern.
              </p>
              <div className="alert-row alert-row-warn">
                Refuses to touch an already-installed <span className="mono">harbory-control-plane.service</span>{" "}
                unless you pass <span className="mono">--force</span> — protects a hand-configured instance from
                getting its env file silently replaced.
              </div>
            </Section>

            <Section id="github" title="Deploying from a repo">
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: "0 0 16px" }}>
                Besides a plain image, a container can be built and deployed straight from a GitHub repo (public
                or private) — the agent clones and builds it locally, no registry involved. This is entirely
                optional and off by default: the control plane needs a GitHub OAuth App connected before any of
                it works.
              </p>
              <ol style={{ fontSize: 14, lineHeight: 1.9, color: "var(--muted)", margin: "0 0 16px", paddingLeft: 20 }}>
                <li>
                  Register an OAuth App on GitHub (Settings → Developer settings → OAuth Apps → New OAuth App).
                  Set <strong>Authorization callback URL</strong> to your control plane's{" "}
                  <span className="mono">/github/oauth/callback</span>. Uncheck{" "}
                  <strong>"Expire user access tokens"</strong> — the control plane doesn't implement the refresh
                  flow yet, so leaving it checked means a connection silently stops working after ~8 hours.
                </li>
                <li>
                  Add four env vars to <span className="mono">/etc/harbory/control-plane.env</span>:{" "}
                  <span className="mono">GITHUB_CLIENT_ID</span>, <span className="mono">GITHUB_CLIENT_SECRET</span>,{" "}
                  <span className="mono">GITHUB_REDIRECT_URI</span> (must match the callback URL exactly), and{" "}
                  <span className="mono">FRONTEND_URL</span> (where the OAuth flow sends the browser back to).
                </li>
                <li><span className="mono">sudo systemctl restart harbory-control-plane</span>.</li>
              </ol>
              <div className="alert-row alert-row-info">
                Without these set, <span className="mono">/github/*</span> just returns 503 — the control plane
                still starts and runs fine either way, this is purely additive.
              </div>
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "16px 0 0" }}>
                Nothing agent-side needs configuring — <span className="mono">install-agent.sh</span> already
                ensures <span className="mono">git</span> is present. A private repo's clone credential is
                embedded into the URL only in the message the control plane sends to the agent at deploy time —
                never written to the database or any file on disk beyond that one ephemeral clone.
              </p>
            </Section>

            <Section id="redeploy" title="Redeploying & re-pairing">
              <ul style={{ fontSize: 14, lineHeight: 1.9, color: "var(--muted)", margin: 0, paddingLeft: 20 }}>
                <li>Both installers are idempotent — re-running rebuilds and reinstalls the binary, leaves an existing env file alone, and restarts the service. That's the whole redeploy path.</li>
                <li>An already-paired agent keeps its credential across redeploys — no need to re-pair after every push.</li>
                <li>If an agent was revoked (or needs to move to a different account), re-pair with a fresh token:</li>
              </ul>
              <div style={{ marginTop: 14 }}>
                <CodeBlock command={REPAIR_COMMAND} label="re-pair command" />
              </div>
            </Section>

            <Section id="troubleshooting" title="Troubleshooting">
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: "0 0 20px" }}>
                Real failure modes, not hypothetical ones — every entry below actually happened while setting this
                up.
              </p>

              <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
                <div className="pixel-panel-sm" style={{ padding: "16px 18px", background: "var(--bg)" }}>
                  <div className="mono" style={{ fontSize: 12.5, fontWeight: 700, marginBottom: 6 }}>
                    E: Unable to locate package harbory-agent
                  </div>
                  <p style={{ fontSize: 13, lineHeight: 1.7, color: "var(--muted)", margin: 0 }}>
                    The apt repo was never registered on this host, or <span className="mono">apt update</span>{" "}
                    hasn't run since it was added. Run the Option B commands above in order — Step 1 (add the
                    repo), <span className="mono">sudo apt update</span>, then Step 2.
                  </p>
                </div>

                <div className="pixel-panel-sm" style={{ padding: "16px 18px", background: "var(--bg)" }}>
                  <div className="mono" style={{ fontSize: 12.5, fontWeight: 700, marginBottom: 6 }}>
                    Could not open lock file /var/lib/dpkg/lock-frontend
                  </div>
                  <p style={{ fontSize: 13, lineHeight: 1.7, color: "var(--muted)", margin: 0 }}>
                    Missing <span className="mono">sudo</span> — <span className="mono">apt install</span> always
                    needs root. <span className="mono">sudo apt install harbory-agent</span>.
                  </p>
                </div>

                <div className="pixel-panel-sm" style={{ padding: "16px 18px", background: "var(--bg)" }}>
                  <div className="mono" style={{ fontSize: 12.5, fontWeight: 700, marginBottom: 6 }}>
                    ERROR harbory_agent::compose: failed to run docker compose ls
                  </div>
                  <p style={{ fontSize: 13, lineHeight: 1.7, color: "var(--muted)", margin: 0 }}>
                    The Docker Compose <em>plugin</em> is a separate package from the daemon on Debian/Ubuntu —{" "}
                    <span className="mono">docker.io</span> doesn't bundle it. Fix:{" "}
                    <span className="mono">sudo apt install docker-compose-v2</span> (or{" "}
                    <span className="mono">docker-compose-plugin</span> if Docker was installed from Docker's own
                    apt repo), then <span className="mono">sudo systemctl restart harbory-agent</span>. Fresh
                    installs pull this in automatically now — only matters for a host that installed the agent
                    before this was fixed.
                  </p>
                </div>

                <div className="pixel-panel-sm" style={{ padding: "16px 18px", background: "var(--bg)" }}>
                  <div className="mono" style={{ fontSize: 12.5, fontWeight: 700, marginBottom: 6 }}>
                    harbory-agent: FAILED — harbory-agent could not run nginx -t through the wrapper
                  </div>
                  <p style={{ fontSize: 13, lineHeight: 1.7, color: "var(--muted)", margin: 0 }}>
                    The installer's own self-test failed — proxy-route deploys won't work until this is fixed.
                    Check the printed output for the actual <span className="mono">nginx -t</span> error, and that{" "}
                    <span className="mono">/etc/sudoers.d/harbory-agent-nginx</span> exists and passed{" "}
                    <span className="mono">visudo -c</span> validation (the installer skips installing it, with a
                    warning, if the generated rule doesn't validate).
                  </p>
                </div>

                <div className="pixel-panel-sm" style={{ padding: "16px 18px", background: "var(--bg)" }}>
                  <div className="mono" style={{ fontSize: 12.5, fontWeight: 700, marginBottom: 6 }}>
                    Agent shows OFFLINE right after pairing
                  </div>
                  <p style={{ fontSize: 13, lineHeight: 1.7, color: "var(--muted)", margin: 0 }}>
                    Check <span className="mono">systemctl status harbory-agent.service</span> first — a common
                    cause is Docker not being reachable at all (the service's own{" "}
                    <span className="mono">ExecStartPre</span> pre-flight check fails fast with Docker's real
                    error rather than a buried panic).{" "}
                    <span className="mono">journalctl -u harbory-agent -f</span> shows the live connection
                    attempts if the service itself is running.
                  </p>
                </div>

                <div className="pixel-panel-sm" style={{ padding: "16px 18px", background: "var(--bg)" }}>
                  <div className="mono" style={{ fontSize: 12.5, fontWeight: 700, marginBottom: 6 }}>
                    A compose stack deploys but its containers never start
                  </div>
                  <p style={{ fontSize: 13, lineHeight: 1.7, color: "var(--muted)", margin: 0 }}>
                    Check for a declared external network the compose file assumes already exists (a common
                    pattern for shared reverse-proxy setups) — the agent creates any missing ones automatically
                    now, but an older agent build won't. Run{" "}
                    <span className="mono">docker network ls</span> on the host to check, and{" "}
                    <span className="mono">sudo apt upgrade harbory-agent</span> (or re-run the install script) to
                    get the fix.
                  </p>
                </div>
              </div>

              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "20px 0 0" }}>
                Not covered here? Open an issue with the exact error and{" "}
                <span className="mono">journalctl -u harbory-agent -n 100 --no-pager</span> output —{" "}
                <a href={`${GITHUB_URL}/issues`} target="_blank" rel="noopener noreferrer" style={{ color: "var(--clay-dark)", fontWeight: 600 }}>
                  github.com/PreetinderSinghBadesha/harbory/issues
                </a>.
              </p>
            </Section>

            <Section id="sprites" title="The robots">
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: "0 0 18px" }}>
                Every agent shows up in the dashboard as a little robot character. Which of the four
                characters you get isn't a setting — it's picked by hashing the agent's id, so the same
                agent always keeps the same look. The color is purely cosmetic; the <em>pose</em> is what
                carries meaning:
              </p>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))", gap: 16 }}>
                <SpriteCard
                  stageClass=""
                  badge="ONLINE"
                  badgeStyle={{ background: "#E4F9EE", color: "var(--hp-dark)", borderColor: "var(--hp-dark)" }}
                  caption="Driving along — connected and reporting heartbeats right now."
                >
                  <>
                    <img className="f1" src={greenDrive1} alt="" />
                    <img className="f2" src={greenDrive2} alt="" />
                  </>
                </SpriteCard>
                <SpriteCard
                  stageClass="sprite-offline"
                  badge="OFFLINE"
                  badgeStyle={{ background: "#F3F0EA", color: "#8A7E72", borderColor: "#8A7E72" }}
                  caption="Grayed out, gently swaying — still paired, but it missed its recent check-ins."
                >
                  <img src={greenHurt} alt="" />
                </SpriteCard>
                <SpriteCard
                  stageClass="sprite-revoked"
                  badge="REVOKED"
                  badgeStyle={{ background: "#FDE8E8", color: "var(--clay)", borderColor: "var(--clay)" }}
                  caption="Red-tinted glitch — access permanently cut off. Re-pair with a fresh token to use it again."
                >
                  <img src={greenHurt} alt="" />
                </SpriteCard>
              </div>
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "18px 0 0" }}>
                Revoked beats everything — a revoked agent never shows the driving animation, even if its
                connection hasn't been dropped yet.
              </p>
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "18px 0 10px" }}>
                The four characters — green, blue, yellow, and red:
              </p>
              <div style={{ display: "flex", gap: 18, flexWrap: "wrap" }}>
                {[
                  { name: "GREEN", img: greenDrive1 },
                  { name: "BLUE", img: blueDrive1 },
                  { name: "YELLOW", img: yellowDrive1 },
                  { name: "RED", img: redDrive1 },
                ].map((c) => (
                  <div key={c.name} className="sprite-sky" style={{ textAlign: "center" }}>
                    <img src={c.img} alt={`${c.name.toLowerCase()} robot`} style={{ width: 56, height: 56, objectFit: "contain" }} />
                    <div className="pixel" style={{ fontSize: 8, color: "var(--muted)", marginTop: 6 }}>{c.name}</div>
                  </div>
                ))}
              </div>
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "18px 0 12px" }}>
                One more image you'll see isn't an agent state at all. In the activity feed on the
                dashboard, a damaged red robot marks a <strong>misuse signal</strong> — security events
                like pairing-token reuse or a credential fingerprint mismatch:
              </p>
              <div
                className="pixel-panel-sm mono"
                style={{ padding: "12px 16px", display: "flex", alignItems: "center", gap: 10, background: "#FFF1F1", maxWidth: 420 }}
              >
                <img src={misuseIcon} alt="" style={{ width: 22, height: 22, objectFit: "contain" }} />
                <span style={{ fontSize: 12, fontWeight: 700, color: "var(--danger-dark)" }}>Misuse detected — check this event.</span>
              </div>
            </Section>

            <div className="pixel-panel-sm" style={{ padding: "20px 22px", background: "var(--bg)", display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: 12 }}>
              <span style={{ fontSize: 13.5, color: "var(--muted)" }}>
                Looking for protocol, security, or reconciliation internals?
              </span>
              <a href={DOCS_FOLDER_URL} target="_blank" rel="noopener noreferrer" className="pixel-btn pixel-btn-ghost pixel-btn-sm">
                FULL DOCS ON GITHUB
              </a>
            </div>
        </div>
      </div>

      <footer style={{ borderTop: "4px solid var(--ink)", background: "var(--panel)" }}>
        <div className="docs-section-inner" style={{ padding: 32, display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: 16 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 9 }}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--muted)" strokeWidth="1.6" strokeLinejoin="round">
              <path d="M12 2.5 L21 7.5 V16.5 L12 21.5 L3 16.5 V7.5 Z" />
              <path d="M3 7.5 L12 12.5 L21 7.5" />
              <path d="M12 12.5 V21.5" />
            </svg>
            <span className="mono" style={{ fontSize: 12, color: "var(--muted)", fontWeight: 600 }}>
              Harbory — open source infrastructure orchestration, built in Rust.
            </span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 22 }}>
            <Link to="/" style={{ fontSize: 13, fontWeight: 600, color: "var(--muted)" }}>Home</Link>
            <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer" style={{ fontSize: 13, fontWeight: 600, color: "var(--muted)" }}>GitHub</a>
            <Link to="/dashboard" style={{ fontSize: 13, fontWeight: 600, color: "var(--muted)" }}>Dashboard</Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
