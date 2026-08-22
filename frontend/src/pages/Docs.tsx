import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { CopyButton } from "../components/CopyButton";
import { GitHubMark } from "../components/GitHubMark";
import { spriteFor } from "../lib/agentSprite";
import "../styles/GameHud.css";
import "./Docs.css";

/** Not a real agent id — just a stable input so the guide character gets
 * one deterministic color (same convention as agent sprites: hashed, not
 * random or configurable) rather than picking arbitrarily. */
const GUIDE_SPRITE = spriteFor("harbory-docs-guide");

const GITHUB_URL = "https://github.com/PreetinderSinghBadesha/harbory";
const DOCS_FOLDER_URL = "https://github.com/PreetinderSinghBadesha/harbory/tree/master/docs";
const INSTALL_SCRIPT_URL = "https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-agent.sh";
const CONTROL_PLANE_SCRIPT_URL = "https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-control-plane.sh";
const INSTALL_COMMAND = `curl -fsSL ${INSTALL_SCRIPT_URL} | bash`;
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
  { id: "redeploy", label: "Redeploying & re-pairing" },
];

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
              Two scripts, deliberately separate: <span className="mono">install-agent.sh</span> sets up everything
              a host needs before it can run an agent at all, and{" "}
              <span className="mono">harbory-agent-pair</span> connects it to a control plane using a short-lived
              pairing token from the dashboard. Both are safe to re-run.
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
                <strong>nginx is optional.</strong> The agent only touches nginx when a reverse-proxy route is
                actually pushed to it. On a host with no nginx installed, the installer skips that setup entirely
                and container-only deployment still works.
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
                Run this on the host you want to manage. No pairing token needed yet — it doesn't start the agent,
                just gets the host ready.
              </p>
              <CodeBlock command={INSTALL_COMMAND} label="install command" />
              <p style={{ fontSize: 13.5, lineHeight: 1.7, color: "var(--muted)", margin: "18px 0 0" }}>
                This builds <span className="mono">harbory-agent</span>, installs it to{" "}
                <span className="mono">/usr/local/bin</span>, and creates:
              </p>
              <ul style={{ fontSize: 13.5, lineHeight: 1.9, color: "var(--muted)", margin: "8px 0 0", paddingLeft: 20 }}>
                <li>a dedicated, unprivileged <span className="mono">harbory-agent</span> system user</li>
                <li>its own data directory for the identity key and stored credential</li>
                <li>a systemd unit (<span className="mono">harbory-agent.service</span>) — installed, not started yet</li>
                <li>the <span className="mono">harbory-agent-pair</span> helper used in the next step</li>
              </ul>
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
