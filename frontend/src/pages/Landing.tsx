import { useEffect, useRef, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import heroImage from "../assets/hero.png";
import "./Landing.css";

const GITHUB_URL = "https://github.com/PreetinderSinghBadesha/harbory";
const DOCS_URL = "https://github.com/PreetinderSinghBadesha/harbory/tree/master/docs";
const INSTALL_COMMAND = `cargo install --git ${GITHUB_URL}.git harbory-agent`;

function GitHubMark({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 .5C5.73.5.98 5.24.98 11.52c0 5.02 3.26 9.28 7.77 10.78.57.1.78-.25.78-.55 0-.27-.01-1.16-.02-2.11-3.16.69-3.83-1.34-3.83-1.34-.52-1.31-1.26-1.66-1.26-1.66-1.03-.7.08-.69.08-.69 1.14.08 1.74 1.17 1.74 1.17 1.01 1.74 2.66 1.24 3.31.95.1-.73.4-1.24.72-1.53-2.52-.29-5.17-1.26-5.17-5.61 0-1.24.44-2.25 1.17-3.05-.12-.29-.51-1.45.11-3.02 0 0 .96-.31 3.13 1.16a10.8 10.8 0 0 1 5.7 0c2.17-1.47 3.13-1.16 3.13-1.16.62 1.57.23 2.73.11 3.02.73.8 1.17 1.81 1.17 3.05 0 4.36-2.66 5.32-5.19 5.6.41.36.77 1.06.77 2.14 0 1.54-.01 2.79-.01 3.17 0 .3.2.66.79.55A10.53 10.53 0 0 0 23.02 11.5C23.02 5.24 18.27.5 12 .5Z" />
    </svg>
  );
}

function ArrowRightIcon() {
  return (
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M5 12h14M13 6l6 6-6 6" />
    </svg>
  );
}

function CopyButton({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard access can be denied by the browser; the command is
      // still visible and selectable, so this is a silent no-op.
    }
  }

  return (
    <button type="button" className="landing-copy-btn" onClick={handleCopy} aria-label={`Copy ${label}`}>
      {copied ? (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M20 6 9 17l-5-5" />
        </svg>
      ) : (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <rect x="8" y="8" width="12" height="12" rx="2" />
          <path d="M4 16V5a1 1 0 0 1 1-1h11" />
        </svg>
      )}
    </button>
  );
}

function Reveal({ children, className }: { children: ReactNode; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { threshold: 0.15 },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const classes = ["landing-reveal", visible ? "landing-reveal-visible" : "", className ?? ""]
    .filter(Boolean)
    .join(" ");

  return (
    <div ref={ref} className={classes}>
      {children}
    </div>
  );
}

export function Landing() {
  return (
    <div className="landing">
      <header className="landing-nav">
        <div className="landing-nav-inner">
          <div className="landing-brand">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="var(--h-accent)" strokeWidth="1.6" strokeLinejoin="round" aria-hidden="true">
              <path d="M12 2.5 L21 7.5 V16.5 L12 21.5 L3 16.5 V7.5 Z" />
              <path d="M3 7.5 L12 12.5 L21 7.5" />
              <path d="M12 12.5 V21.5" />
            </svg>
            <span>Harbory</span>
          </div>
          <nav className="landing-nav-links">
            <a href="#why">Why Harbory</a>
            <a href="#architecture">Architecture</a>
            <a href="#features">Features</a>
            <a href={DOCS_URL} target="_blank" rel="noopener noreferrer">Docs</a>
          </nav>
          <div className="landing-nav-actions">
            <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer" className="landing-btn landing-btn-ghost">
              <GitHubMark />
              GitHub
            </a>
            <Link to="/dashboard" className="landing-btn landing-btn-primary">Open dashboard</Link>
          </div>
        </div>
      </header>

      <section className="landing-hero landing-dotgrid-surface">
        <div className="landing-dotgrid-bg" aria-hidden="true" />
        <div className="landing-hero-grid landing-dotgrid-content">
          <div className="landing-hero-copy">
            <div className="landing-eyebrow">Open source · Rust</div>
            <h1 className="landing-h1">Infrastructure orchestration without the Kubernetes tax.</h1>
            <p className="landing-lede">
              Harbory is a distributed control plane and agent system, written entirely in Rust. Deploy containers,
              manage reverse proxies, and coordinate remote VMs from one dashboard — no cluster to run, no YAML to
              write.
            </p>
            <div className="landing-cta-row">
              <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer" className="landing-btn landing-btn-primary landing-btn-lg">
                <GitHubMark />
                View on GitHub
              </a>
              <Link to="/dashboard" className="landing-btn landing-btn-ghost landing-btn-lg">
                Open dashboard
                <ArrowRightIcon />
              </Link>
            </div>
            <div>
              <div className="landing-label">Install the agent</div>
              <div className="landing-code-box">
                <span className="landing-prompt">$</span>
                <code>{INSTALL_COMMAND}</code>
                <CopyButton text={INSTALL_COMMAND} label="install command" />
              </div>
            </div>
          </div>
          <div className="landing-hero-art">
            <div className="landing-hero-glow" aria-hidden="true" />
            <img src={heroImage} alt="" className="landing-hero-image" />
          </div>
        </div>
      </section>

      <section id="why" className="landing-why">
        <Reveal className="landing-section-inner">
          <div className="landing-section-head">
            <div className="landing-eyebrow">Why Harbory</div>
            <h2>Built for a handful of VMs, not a datacenter.</h2>
          </div>
          <div className="landing-why-grid">
            <div className="landing-why-cell">
              <div className="landing-mono-num">01</div>
              <h3>No cluster to operate</h3>
              <p>
                No etcd, kube-apiserver, or controller-manager. One control plane process and a small agent binary,
                talking over a single persistent gRPC stream.
              </p>
            </div>
            <div className="landing-why-cell">
              <div className="landing-mono-num">02</div>
              <h3>No YAML sprawl</h3>
              <p>
                Declare desired container and proxy state from the dashboard or a plain JSON API — not a stack of
                manifests, CRDs, and Helm charts.
              </p>
            </div>
            <div className="landing-why-cell">
              <div className="landing-mono-num">03</div>
              <h3>Pair, don&apos;t provision</h3>
              <p>
                A short-lived pairing token and one <code>cargo install</code> get an agent talking to the control
                plane — no cluster bootstrap, no control-node quorum.
              </p>
            </div>
          </div>
        </Reveal>
      </section>

      <section id="architecture">
      <Reveal className="landing-section-inner landing-architecture">
        <div className="landing-section-head">
          <div className="landing-eyebrow">Architecture</div>
          <h2>One control plane. Many agents.</h2>
          <p className="landing-section-sub">
            A central server coordinates lightweight agents over an authenticated, persistent connection — each
            agent runs commands and reports state on its own VM.
          </p>
        </div>

        <div className="landing-arch-grid">
          <div className="landing-arch-card">
            <div className="landing-arch-card-head">
              <div className="landing-arch-icon">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--h-accent)" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                  <rect x="3" y="4" width="18" height="6" rx="1.5" />
                  <rect x="3" y="14" width="18" height="6" rx="1.5" />
                  <circle cx="7" cy="7" r="0.6" fill="var(--h-accent)" stroke="none" />
                  <circle cx="7" cy="17" r="0.6" fill="var(--h-accent)" stroke="none" />
                </svg>
              </div>
              <h3>Control Plane</h3>
            </div>
            <ul>
              <li>Web dashboard</li>
              <li>Auth &amp; accounts</li>
              <li>Pairing tokens</li>
              <li>Agent registry</li>
              <li>Command dispatch</li>
              <li>State store</li>
            </ul>
          </div>

          <div className="landing-arch-connector" aria-hidden="true">
            <div className="landing-arch-connector-line" />
            <ArrowRightIcon />
            <div className="landing-arch-connector-label">
              gRPC
              <br />
              bi-directional
              <br />
              stream
            </div>
          </div>

          <div className="landing-arch-card">
            <div className="landing-arch-card-head">
              <div className="landing-arch-icon">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--h-accent)" strokeWidth="1.7" strokeLinejoin="round" aria-hidden="true">
                  <path d="M12 3 L20 7.5 V16.5 L12 21 L4 16.5 V7.5 Z" />
                  <path d="M4 7.5 L12 12 L20 7.5" />
                  <path d="M12 12 V21" />
                </svg>
              </div>
              <h3>Agent</h3>
            </div>
            <ul>
              <li>Local Ed25519 keypair</li>
              <li>Container management</li>
              <li>Nginx reconfiguration</li>
              <li>Heartbeat &amp; state reports</li>
            </ul>
          </div>
        </div>
      </Reveal>
      </section>

      <section id="features" className="landing-features">
        <Reveal className="landing-section-inner">
          <div className="landing-section-head">
            <div className="landing-eyebrow">Features</div>
            <h2>Everything a small fleet actually needs.</h2>
          </div>
          <div className="landing-feature-grid">
            <div className="landing-feature-card">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="var(--h-accent)" strokeWidth="1.6" strokeLinejoin="round" aria-hidden="true">
                <path d="M12 3 L20 7.5 V16.5 L12 21 L4 16.5 V7.5 Z" />
                <path d="M4 7.5 L12 12 L20 7.5" />
                <path d="M12 12 V21" />
              </svg>
              <h3>Container deployment</h3>
              <p>
                Declare desired container state; the agent reconciles it against Docker automatically — deployed,
                redeployed, or removed, converged on every heartbeat.
              </p>
            </div>
            <div className="landing-feature-card">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="var(--h-accent)" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M4 7h11a4 4 0 0 1 0 8H9" />
                <path d="M12 12l-3 3 3 3" />
              </svg>
              <h3>Reverse proxy management</h3>
              <p>
                Nginx routes are templated, validated in place, and only reloaded if they pass — a bad config never
                reaches live traffic.
              </p>
            </div>
            <div className="landing-feature-card">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="var(--h-accent)" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M12 3l7 3v6c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6l7-3Z" />
                <path d="M9 12l2 2 4-4" />
              </svg>
              <h3>Ed25519 pairing &amp; auth</h3>
              <p>
                Agents prove key possession on every connection, not just a bearer credential — short-lived pairing
                tokens, signed credentials, challenge/response.
              </p>
            </div>
            <div className="landing-feature-card">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="var(--h-accent)" strokeWidth="1.6" strokeLinejoin="round" aria-hidden="true">
                <rect x="3" y="3" width="8" height="8" rx="1.3" />
                <rect x="13" y="3" width="8" height="5" rx="1.3" />
                <rect x="13" y="10" width="8" height="11" rx="1.3" />
                <rect x="3" y="13" width="8" height="8" rx="1.3" />
              </svg>
              <h3>Web dashboard</h3>
              <p>
                Pair agents, deploy containers and routes, and watch a live security and activity feed — all from
                one account.
              </p>
            </div>
          </div>
        </Reveal>
      </section>

      <Reveal className="landing-section-inner">
        <div className="landing-cta-band landing-dotgrid-surface">
          <div className="landing-dotgrid-bg" aria-hidden="true" />
          <div className="landing-dotgrid-content">
            <div className="landing-eyebrow">Get started</div>
            <h2>An agent running in two commands.</h2>
            <div className="landing-cta-steps">
              <div className="landing-code-box landing-code-box-step">
                <span className="landing-prompt">1</span>
                <code>{INSTALL_COMMAND}</code>
                <CopyButton text={INSTALL_COMMAND} label="install command" />
              </div>
              <div className="landing-code-box landing-code-box-step">
                <span className="landing-prompt">2</span>
                <code>harbory-agent &lt;pairing-token&gt;</code>
                <CopyButton text="harbory-agent <pairing-token>" label="run command" />
              </div>
            </div>
            <Link to="/dashboard" className="landing-btn landing-btn-primary landing-btn-lg">
              Open dashboard to generate a token
              <ArrowRightIcon />
            </Link>
          </div>
        </div>
      </Reveal>

      <footer className="landing-footer">
        <div className="landing-section-inner landing-footer-inner">
          <div className="landing-footer-brand">
            <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="var(--h-ink-faint)" strokeWidth="1.6" strokeLinejoin="round" aria-hidden="true">
              <path d="M12 2.5 L21 7.5 V16.5 L12 21.5 L3 16.5 V7.5 Z" />
              <path d="M3 7.5 L12 12.5 L21 7.5" />
              <path d="M12 12.5 V21.5" />
            </svg>
            <span>Harbory — open source infrastructure orchestration, built in Rust.</span>
          </div>
          <div className="landing-footer-links">
            <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer">GitHub</a>
            <a href={DOCS_URL} target="_blank" rel="noopener noreferrer">Docs</a>
            <Link to="/dashboard">Dashboard</Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
