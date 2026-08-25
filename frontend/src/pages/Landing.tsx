import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { CopyButton } from "../components/CopyButton";
import { GitHubMark } from "../components/GitHubMark";
import greenDrive1 from "../assets/sprites/robot_greenDrive1.png";
import greenDrive2 from "../assets/sprites/robot_greenDrive2.png";
import greenBody from "../assets/sprites/robot_greenBody.png";
import yellowDrive1 from "../assets/sprites/robot_yellowDrive1.png";
import blueJump from "../assets/sprites/robot_blueJump.png";
import "../styles/GameHud.css";
import "./Landing.css";

const GITHUB_URL = "https://github.com/PreetinderSinghBadesha/harbory";
const INSTALL_SCRIPT_URL = "https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-agent.sh";
const INSTALL_COMMAND = `curl -fsSL ${INSTALL_SCRIPT_URL} | bash`;
const PAIR_COMMAND = "sudo harbory-agent-pair <pairing-token>";

function Reveal({
  children,
  className,
  style,
}: {
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
}) {
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

  const classes = ["reveal", visible ? "reveal-visible" : "", className ?? ""].filter(Boolean).join(" ");

  return (
    <div ref={ref} className={classes} style={style}>
      {children}
    </div>
  );
}

export function Landing() {
  return (
    <div className="game-hud">
      <header style={{ background: "var(--panel)", borderBottom: "4px solid var(--ink)", position: "sticky", top: 0, zIndex: 20 }}>
        <div className="landing-section-inner" style={{ height: 94, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div className="pixel-panel-sm" style={{ width: 32, height: 32, background: "var(--clay)", display: "flex", alignItems: "center", justifyContent: "center" }}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round">
                <path d="M12 2.5 L21 7.5 V16.5 L12 21.5 L3 16.5 V7.5 Z" />
                <path d="M3 7.5 L12 12.5 L21 7.5" />
                <path d="M12 12.5 V21.5" />
              </svg>
            </div>
            <span className="pixel" style={{ fontSize: 14 }}>HARBORY</span>
          </div>
          <nav className="landing-nav-links">
            <a href="#why" style={{ fontSize: 14, fontWeight: 600, color: "var(--muted)" }}>Why Harbory</a>
            <a href="#architecture" style={{ fontSize: 14, fontWeight: 600, color: "var(--muted)" }}>Architecture</a>
            <a href="#features" style={{ fontSize: 14, fontWeight: 600, color: "var(--muted)" }}>Features</a>
            <Link to="/docs" style={{ fontSize: 14, fontWeight: 600, color: "var(--muted)" }}>Docs</Link>
          </nav>
          <div className="landing-nav-actions" style={{ display: "flex", alignItems: "center", gap: 14 }}>
            <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer" className="pixel-btn pixel-btn-ghost pixel-btn-sm">
              <GitHubMark />
              <span className="landing-gh-label">GITHUB</span>
            </a>
            <Link to="/dashboard" className="pixel-btn pixel-btn-sm">DASHBOARD</Link>
          </div>
        </div>
      </header>

      {/* HERO */}
      <div className="landing-section-inner" style={{ padding: "88px 32px 80px" }}>
        <div className="landing-hero-grid">
          <div>
            <div className="eyebrow">OPEN SOURCE · RUST</div>
            <h1 className="landing-h1" style={{ fontFamily: "var(--font-sans)", fontSize: 48, fontWeight: 800, lineHeight: 1.08, letterSpacing: "-0.02em", margin: "0 0 20px" }}>
              Infrastructure orchestration without the Kubernetes tax.
            </h1>
            <p style={{ fontSize: 16.5, lineHeight: 1.6, color: "var(--muted)", maxWidth: "50ch", margin: "0 0 30px" }}>
              Harbory is a distributed control plane and agent system, written entirely in Rust. Deploy containers,
              manage reverse proxies, and coordinate remote VMs from one dashboard — no cluster to run, no YAML to
              write.
            </p>
            <div style={{ display: "flex", gap: 12, marginBottom: 28, flexWrap: "wrap" }}>
              <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer" className="pixel-btn">
                <GitHubMark size={15} />
                VIEW ON GITHUB
              </a>
              <Link to="/dashboard" className="pixel-btn pixel-btn-ghost">OPEN DASHBOARD</Link>
            </div>
            <div>
              <div className="mono" style={{ fontSize: 11, color: "var(--muted)", marginBottom: 8, fontWeight: 600 }}>Install the agent</div>
              <div className="pixel-panel-sm" style={{ display: "flex", alignItems: "center", gap: 12, padding: "13px 14px", background: "#FFF9EE" }}>
                <span className="mono" style={{ color: "var(--clay-dark)", fontSize: 13, fontWeight: 700 }}>$</span>
                <code className="mono" style={{ flex: 1, minWidth: 0, fontSize: 12.5, overflowX: "auto", whiteSpace: "nowrap" }}>{INSTALL_COMMAND}</code>
                <CopyButton text={INSTALL_COMMAND} label="install command" />
              </div>
            </div>
          </div>

          <div className="landing-hero-art sprite-sky" style={{ padding: "20px 20px 14px" }}>
            <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "center", gap: 18, marginBottom: 14 }}>
              <div className="sprite-stage sprite-deco-bob" style={{ width: 76, height: 76, animationDelay: "0.3s" }}>
                <img src={yellowDrive1} alt="" style={{ position: "relative", width: "100%", height: "100%", objectFit: "contain" }} />
              </div>
              <div className="sprite-stage" style={{ width: 96, height: 96 }}>
                <img className="f1" src={greenDrive1} alt="" />
                <img className="f2" src={greenDrive2} alt="" />
              </div>
              <div className="sprite-stage sprite-deco-bob" style={{ width: 76, height: 76 }}>
                <img src={blueJump} alt="" style={{ position: "relative", width: "100%", height: "100%", objectFit: "contain" }} />
              </div>
            </div>
            <div className="ground" />
          </div>
        </div>
      </div>

      {/* WHY HARBORY */}
      <div id="why" style={{ borderTop: "4px solid var(--ink)", background: "var(--panel)", padding: "88px 0" }}>
        <Reveal className="landing-section-inner">
          <div style={{ maxWidth: 640, marginBottom: 44 }}>
            <div className="eyebrow">WHY HARBORY</div>
            <h2 style={{ fontFamily: "var(--font-sans)", fontSize: 30, fontWeight: 800, letterSpacing: "-0.015em", margin: 0 }}>
              Built for a handful of VMs, not a datacenter.
            </h2>
          </div>
          <div className="landing-why-grid">
            <div className="pixel-panel-sm" style={{ padding: "26px 22px", background: "var(--bg)" }}>
              <div className="pixel" style={{ color: "var(--muted)", fontSize: 11, marginBottom: 14 }}>01</div>
              <h3 style={{ fontSize: 16.5, fontWeight: 800, margin: "0 0 10px" }}>No cluster to operate</h3>
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: 0 }}>
                No etcd, kube-apiserver, or controller-manager. One control plane process and a small agent binary,
                talking over a single persistent gRPC stream.
              </p>
            </div>
            <div className="pixel-panel-sm" style={{ padding: "26px 22px", background: "var(--bg)" }}>
              <div className="pixel" style={{ color: "var(--muted)", fontSize: 11, marginBottom: 14 }}>02</div>
              <h3 style={{ fontSize: 16.5, fontWeight: 800, margin: "0 0 10px" }}>No YAML sprawl</h3>
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: 0 }}>
                Declare desired container and proxy state from the dashboard or a plain JSON API — not a stack of
                manifests, CRDs, and Helm charts.
              </p>
            </div>
            <div className="pixel-panel-sm" style={{ padding: "26px 22px", background: "var(--bg)" }}>
              <div className="pixel" style={{ color: "var(--muted)", fontSize: 11, marginBottom: 14 }}>03</div>
              <h3 style={{ fontSize: 16.5, fontWeight: 800, margin: "0 0 10px" }}>Pair, don&apos;t provision</h3>
              <p style={{ fontSize: 14, lineHeight: 1.6, color: "var(--muted)", margin: 0 }}>
                One command sets up docker group and nginx permissions automatically; a short-lived pairing token
                from the dashboard is all a second command needs to get the agent talking to the control plane —
                no cluster bootstrap, no control-node quorum.
              </p>
            </div>
          </div>
        </Reveal>
      </div>

      {/* ARCHITECTURE */}
      <section id="architecture">
        <Reveal className="landing-section-inner" style={{ padding: "96px 32px" }}>
          <div style={{ maxWidth: 640, marginBottom: 48 }}>
            <div className="eyebrow">ARCHITECTURE</div>
            <h2 style={{ fontFamily: "var(--font-sans)", fontSize: 30, fontWeight: 800, letterSpacing: "-0.015em", margin: "0 0 14px" }}>
              One control plane. Many agents.
            </h2>
            <p style={{ fontSize: 14.5, lineHeight: 1.6, color: "var(--muted)", margin: 0 }}>
              A central server coordinates lightweight agents over an authenticated, persistent connection — each
              agent runs commands and reports state on its own VM.
            </p>
          </div>

          <div className="landing-arch-grid">
            <div className="pixel-panel" style={{ padding: 28 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 18 }}>
                <div className="pixel-panel-sm" style={{ width: 32, height: 32, background: "var(--blue)", display: "flex", alignItems: "center", justifyContent: "center" }}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                    <rect x="3" y="4" width="18" height="6" rx="1.5" />
                    <rect x="3" y="14" width="18" height="6" rx="1.5" />
                  </svg>
                </div>
                <h3 style={{ fontSize: 16.5, fontWeight: 800, margin: 0 }}>Control Plane</h3>
              </div>
              <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
                {["Web dashboard", "Auth & accounts", "Pairing tokens", "Agent registry", "Command dispatch", "State store"].map((item) => (
                  <li key={item} className="mono" style={{ fontSize: 12.5, color: "var(--muted)", padding: "8px 6px", borderBottom: "2px dashed var(--line)" }}>
                    {item}
                  </li>
                ))}
              </ul>
            </div>

            <div className="landing-arch-connector" style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", padding: "20px 10px" }}>
              <div style={{ width: "100%", height: 0, borderTop: "3px dashed var(--ink)" }} />
              <div className="pixel" style={{ fontSize: 8, color: "var(--muted)", textAlign: "center", marginTop: 14, lineHeight: 1.8 }}>
                gRPC
                <br />
                STREAM
              </div>
            </div>

            <div className="pixel-panel" style={{ padding: 28, textAlign: "center" }}>
              <div className="sprite-stage sprite-deco-bob" style={{ width: 64, height: 64, margin: "0 auto 10px" }}>
                <img src={greenBody} alt="" style={{ position: "relative", width: "100%", height: "100%", objectFit: "contain" }} />
              </div>
              <h3 style={{ fontSize: 16.5, fontWeight: 800, margin: "0 0 14px" }}>Agent</h3>
              <ul style={{ listStyle: "none", margin: 0, padding: 0, textAlign: "left" }}>
                {["Local Ed25519 keypair", "Container management", "Nginx reconfiguration", "Heartbeat & state reports"].map((item) => (
                  <li key={item} className="mono" style={{ fontSize: 12.5, color: "var(--muted)", padding: "8px 6px", borderBottom: "2px dashed var(--line)" }}>
                    {item}
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </Reveal>
      </section>

      {/* FEATURES */}
      <div id="features" style={{ borderTop: "4px solid var(--ink)", background: "var(--panel)", padding: "96px 0" }}>
        <Reveal className="landing-section-inner">
          <div style={{ maxWidth: 640, marginBottom: 44 }}>
            <div className="eyebrow">FEATURES</div>
            <h2 style={{ fontFamily: "var(--font-sans)", fontSize: 30, fontWeight: 800, letterSpacing: "-0.015em", margin: 0 }}>
              Everything a small fleet actually needs.
            </h2>
          </div>
          <div className="landing-feature-grid">
            <div className="pixel-panel-sm" style={{ padding: 26, background: "var(--bg)" }}>
              <div className="pixel-panel-sm" style={{ width: 38, height: 38, background: "var(--clay)", display: "flex", alignItems: "center", justifyContent: "center", marginBottom: 16 }}>
                <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="1.7" strokeLinejoin="round">
                  <path d="M12 3 L20 7.5 V16.5 L12 21 L4 16.5 V7.5 Z" />
                  <path d="M4 7.5 L12 12 L20 7.5" />
                  <path d="M12 12 V21" />
                </svg>
              </div>
              <h3 style={{ fontSize: 15.5, fontWeight: 800, margin: "0 0 8px" }}>Container deployment</h3>
              <p style={{ fontSize: 13.5, lineHeight: 1.6, color: "var(--muted)", margin: 0 }}>
                Declare desired container state; the agent reconciles it against Docker automatically — deployed,
                redeployed, or removed, converged on every heartbeat.
              </p>
            </div>

            <div className="pixel-panel-sm" style={{ padding: 26, background: "var(--bg)" }}>
              <div className="pixel-panel-sm" style={{ width: 38, height: 38, background: "var(--blue)", display: "flex", alignItems: "center", justifyContent: "center", marginBottom: 16 }}>
                <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M4 7h11a4 4 0 0 1 0 8H9" />
                  <path d="M12 12l-3 3 3 3" />
                </svg>
              </div>
              <h3 style={{ fontSize: 15.5, fontWeight: 800, margin: "0 0 8px" }}>Reverse proxy management</h3>
              <p style={{ fontSize: 13.5, lineHeight: 1.6, color: "var(--muted)", margin: 0 }}>
                Nginx routes are templated, validated in place, and only reloaded if they pass — a bad config never
                reaches live traffic.
              </p>
            </div>

            <div className="pixel-panel-sm" style={{ padding: 26, background: "var(--bg)" }}>
              <div className="pixel-panel-sm" style={{ width: 38, height: 38, background: "var(--hp)", display: "flex", alignItems: "center", justifyContent: "center", marginBottom: 16 }}>
                <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M12 3l7 3v6c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6l7-3Z" />
                  <path d="M9 12l2 2 4-4" />
                </svg>
              </div>
              <h3 style={{ fontSize: 15.5, fontWeight: 800, margin: "0 0 8px" }}>Ed25519 pairing &amp; auth</h3>
              <p style={{ fontSize: 13.5, lineHeight: 1.6, color: "var(--muted)", margin: 0 }}>
                Agents prove key possession on every connection, not just a bearer credential — short-lived pairing
                tokens, signed credentials, challenge/response.
              </p>
            </div>

            <div className="pixel-panel-sm" style={{ padding: 26, background: "var(--bg)" }}>
              <div className="pixel-panel-sm" style={{ width: 38, height: 38, background: "var(--gold)", display: "flex", alignItems: "center", justifyContent: "center", marginBottom: 16 }}>
                <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="1.7" strokeLinejoin="round">
                  <rect x="3" y="3" width="8" height="8" rx="1.3" />
                  <rect x="13" y="3" width="8" height="5" rx="1.3" />
                  <rect x="13" y="10" width="8" height="11" rx="1.3" />
                  <rect x="3" y="13" width="8" height="8" rx="1.3" />
                </svg>
              </div>
              <h3 style={{ fontSize: 15.5, fontWeight: 800, margin: "0 0 8px" }}>Web dashboard</h3>
              <p style={{ fontSize: 13.5, lineHeight: 1.6, color: "var(--muted)", margin: 0 }}>
                Pair agents, deploy containers and routes, and watch a live security and activity feed — all from one
                account.
              </p>
            </div>
          </div>
        </Reveal>
      </div>

      {/* CTA */}
      <Reveal className="landing-section-inner" style={{ padding: "96px 32px" }}>
        <div className="pixel-panel" style={{ padding: 0, overflow: "hidden" }}>
          <div className="sprite-sky" style={{ borderRadius: 0, textAlign: "center", padding: "56px 32px 40px" }}>
            <div className="eyebrow" style={{ display: "inline-block" }}>GET STARTED</div>
            <h2 style={{ fontFamily: "var(--font-sans)", fontSize: 26, fontWeight: 800, letterSpacing: "-0.015em", margin: "0 0 28px" }}>
              An agent running in two commands.
            </h2>
            <div style={{ maxWidth: 560, margin: "0 auto 26px", display: "flex", flexDirection: "column", gap: 10 }}>
              <div className="pixel-panel-sm" style={{ display: "flex", alignItems: "center", gap: 10, padding: "12px 14px", background: "#FFF9EE", textAlign: "left" }}>
                <span className="pixel" style={{ color: "var(--clay-dark)", fontSize: 9 }}>1</span>
                <code className="mono" style={{ flex: 1, minWidth: 0, fontSize: 12, overflowX: "auto", whiteSpace: "nowrap" }}>{INSTALL_COMMAND}</code>
                <CopyButton text={INSTALL_COMMAND} label="install command" />
              </div>
              <div className="pixel-panel-sm" style={{ display: "flex", alignItems: "center", gap: 10, padding: "12px 14px", background: "#FFF9EE", textAlign: "left" }}>
                <span className="pixel" style={{ color: "var(--clay-dark)", fontSize: 9 }}>2</span>
                <code className="mono" style={{ flex: 1, minWidth: 0, fontSize: 12 }}>{PAIR_COMMAND}</code>
                <CopyButton text={PAIR_COMMAND} label="pair command" />
              </div>
            </div>
            <Link to="/dashboard" className="pixel-btn">OPEN DASHBOARD TO GENERATE A TOKEN</Link>
          </div>
        </div>
      </Reveal>

      {/* FOOTER */}
      <footer style={{ borderTop: "4px solid var(--ink)", background: "var(--panel)" }}>
        <div className="landing-section-inner" style={{ padding: 32, display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: 16 }}>
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
            <a href="mailto:preetindersingh13per@gmail.com" style={{ fontSize: 13, fontWeight: 600, color: "var(--muted)" }}>preetindersingh13per@gmail.com</a>
            <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer" style={{ fontSize: 13, fontWeight: 600, color: "var(--muted)" }}>GitHub</a>
            <Link to="/docs" style={{ fontSize: 13, fontWeight: 600, color: "var(--muted)" }}>Docs</Link>
            <Link to="/dashboard" style={{ fontSize: 13, fontWeight: 600, color: "var(--muted)" }}>Dashboard</Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
