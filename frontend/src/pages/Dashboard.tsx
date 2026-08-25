import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { apiFetch } from "../lib/api";
import { supabase } from "../lib/supabase";
import { useAuth } from "../context/AuthContext";
import { misuseIcon, situationFor, spriteFor } from "../lib/agentSprite";
import { fetchGitHubConnection } from "../lib/github";
import { CopyButton } from "../components/CopyButton";
import "../styles/GameHud.css";

const INSTALL_COMMAND = "curl -fsSL https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-agent.sh | bash";

interface AgentSummary {
  id: string;
  status: string;
  online: boolean;
  last_heartbeat_at: string | null;
}

interface PairingToken {
  token: string;
  expires_at: string;
}

interface SecurityEvent {
  event_type: string;
  agent_id: string | null;
  detail: Record<string, unknown>;
  created_at: string;
  is_misuse_signal: boolean;
}

const EVENT_LABELS: Record<string, string> = {
  pairing_success: "Agent paired",
  pairing_token_reuse: "Pairing token reuse detected",
  pairing_token_expired: "Expired pairing token used",
  credential_fingerprint_mismatch: "Credential fingerprint mismatch",
  agent_revoked: "Agent revoked",
  agent_deleted: "Agent deleted",
  github_connected: "GitHub account connected",
  github_disconnected: "GitHub account disconnected",
};


function AgentSprite({ agent }: { agent: AgentSummary }) {
  const sprite = spriteFor(agent.id);
  const situation = situationFor(agent);
  return (
    <div className="sprite-sky">
      <div className={`sprite-stage sprite-${situation}`}>
        {situation === "online" ? (
          <>
            <img className="f1" src={sprite.drive1} alt="" />
            <img className="f2" src={sprite.drive2} alt="" />
          </>
        ) : (
          <img src={sprite.hurt} alt="" style={{ position: "relative", width: "100%", height: "100%", objectFit: "contain" }} />
        )}
      </div>
      <div className="ground" style={{ margin: "8px 0 0" }} />
    </div>
  );
}

function HpBar({ online }: { online: boolean }) {
  return (
    <div className="hp-bar" style={{ marginBottom: 8 }}>
      {Array.from({ length: 6 }).map((_, i) => (
        <span key={i} className={online ? "hp-pip hp-pip-on" : "hp-pip"} />
      ))}
    </div>
  );
}

function formatCountdown(ms: number): string {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(totalSeconds / 60);
  const s = totalSeconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

/** The countdown and its progress bar are both driven by the token's real
 * `expires_at` — `issuedAt` is just the moment this component first saw
 * the token, captured once, so the bar's total length reflects the
 * server's real TTL rather than a guessed constant. */
function PairingTokenCard({ token, issuedAt }: { token: PairingToken; issuedAt: number }) {
  const expiresAt = useMemo(() => new Date(token.expires_at).getTime(), [token.expires_at]);
  const totalMs = Math.max(expiresAt - issuedAt, 1);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  const remainingMs = Math.max(expiresAt - now, 0);
  const progress = Math.max(0, Math.min(100, (remainingMs / totalMs) * 100));
  const expired = remainingMs <= 0;
  const pairCommand = `sudo harbory-agent-pair ${token.token}`;

  return (
    <div style={{ flex: 1, minWidth: 300 }}>
      <div
        className="pixel-panel-sm mono"
        style={{
          display: "inline-block",
          padding: "10px 14px",
          fontSize: 13,
          fontWeight: 700,
          color: "var(--clay-dark)",
          background: "#FFF9EE",
          marginBottom: 12,
          wordBreak: "break-all",
        }}
      >
        {token.token}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <div style={{ flex: 1, height: 10, background: "var(--line)", border: "2px solid var(--ink)" }}>
          <div
            style={{
              width: `${progress}%`,
              height: "100%",
              background: expired ? "var(--muted)" : "linear-gradient(180deg, #FFE07A 0%, var(--gold) 100%)",
            }}
          />
        </div>
        <span className="mono" style={{ fontSize: 11, color: "var(--muted)", fontWeight: 700, whiteSpace: "nowrap" }}>
          {expired ? "EXPIRED" : formatCountdown(remainingMs)}
        </span>
      </div>
      <div style={{ fontSize: 10.5, color: "var(--muted)", marginBottom: 4, fontWeight: 600 }}>
        No agent installed on the host yet? <span className="mono">{INSTALL_COMMAND}</span> first, then:
      </div>
      <div
        className="mono"
        style={{ fontSize: 11, color: "var(--muted)", wordBreak: "break-all", display: "flex", alignItems: "flex-start", gap: 6 }}
      >
        <span style={{ flex: 1, minWidth: 0 }}>{pairCommand}</span>
        <CopyButton text={pairCommand} label="pair command" />
      </div>
    </div>
  );
}

export function Dashboard() {
  const queryClient = useQueryClient();
  const { session } = useAuth();

  const { data: agents, isLoading } = useQuery({
    queryKey: ["agents"],
    queryFn: () => apiFetch<AgentSummary[]>("/agents"),
    refetchInterval: 5000, // matches the ~10s heartbeat cadence closely enough for a dashboard
  });

  const [pairingToken, setPairingToken] = useState<PairingToken | null>(null);
  const [pairingIssuedAt, setPairingIssuedAt] = useState(0);
  const issueToken = useMutation({
    mutationFn: () => apiFetch<PairingToken>("/pairing-tokens", { method: "POST", body: JSON.stringify({}) }),
    onSuccess: (data) => {
      setPairingToken(data);
      setPairingIssuedAt(Date.now());
    },
  });

  // "Revoke" on the dashboard matches the agent page: it deletes the agent
  // outright (DELETE /agents/:id), so it always goes through a confirm.
  const revoke = useMutation({
    mutationFn: (agentId: string) => apiFetch<void>(`/agents/${agentId}`, { method: "DELETE" }),
    onSuccess: () => {
      setConfirmRevokeId(null);
      queryClient.invalidateQueries({ queryKey: ["agents"] });
    },
    onError: () => setConfirmRevokeId(null),
  });
  const [confirmRevokeId, setConfirmRevokeId] = useState<string | null>(null);

  const { data: securityEvents } = useQuery({
    queryKey: ["security-events"],
    queryFn: () => apiFetch<SecurityEvent[]>("/security-events"),
    refetchInterval: 10000,
  });

  const { data: github, isLoading: githubLoading } = useQuery({
    queryKey: ["github-connection"],
    queryFn: fetchGitHubConnection,
  });

  const connectGithub = useMutation({
    mutationFn: () => apiFetch<{ authorize_url: string }>("/github/oauth/start", { method: "POST" }),
    onSuccess: (data) => {
      // A real navigation, not a fetch — GitHub's own redirect back to
      // the control plane is what completes the OAuth round trip.
      window.location.href = data.authorize_url;
    },
  });

  const disconnectGithub = useMutation({
    mutationFn: () => apiFetch<void>("/github/connection", { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["github-connection"] }),
  });

  /** `github_oauth_callback` (control-plane) redirects back here with
   * ?github=connected|error once the OAuth round trip finishes — this
   * picks that up once, refreshes the connection query, and strips the
   * param so a page refresh doesn't re-show the banner. */
  const [githubCallbackResult, setGithubCallbackResult] = useState<"connected" | "error" | null>(null);
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const result = params.get("github");
    if (result === "connected" || result === "error") {
      setGithubCallbackResult(result);
      queryClient.invalidateQueries({ queryKey: ["github-connection"] });
      params.delete("github");
      const query = params.toString();
      window.history.replaceState(null, "", window.location.pathname + (query ? `?${query}` : ""));
    }
  }, [queryClient]);

  const onlineCount = agents?.filter((a) => a.online).length ?? 0;

  return (
    <div className="game-hud">
      <header style={{ background: "var(--panel)", borderBottom: "4px solid var(--ink)", padding: "20px 32px" }}>
        <div style={{ maxWidth: 1180, margin: "0 auto" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 18 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <div
                className="pixel-panel-sm"
                style={{ width: 36, height: 36, background: "var(--clay)", display: "flex", alignItems: "center", justifyContent: "center" }}
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round">
                  <path d="M12 2.5 L21 7.5 V16.5 L12 21.5 L3 16.5 V7.5 Z" />
                  <path d="M3 7.5 L12 12.5 L21 7.5" />
                  <path d="M12 12.5 V21.5" />
                </svg>
              </div>
              <span className="pixel" style={{ fontSize: 16 }}>HARBORY</span>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
              {session?.user.email && (
                <span className="mono" style={{ fontSize: 12, color: "var(--muted)", fontWeight: 600 }}>
                  PLAYER: {session.user.email}
                </span>
              )}
              <button type="button" className="pixel-btn pixel-btn-ghost pixel-btn-sm" onClick={() => supabase.auth.signOut()}>
                SIGN OUT
              </button>
            </div>
          </div>

          <div
            className="pixel-panel-sm"
            style={{
              background: "linear-gradient(180deg, #35291C 0%, var(--ink) 100%)",
              padding: "10px 16px",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <span className="pixel" style={{ fontSize: 10, color: "var(--gold)" }}>
              AGENTS ONLINE: {onlineCount}/{agents?.length ?? 0}
            </span>
            {agents && agents.length > 0 && (
              <div style={{ display: "flex", gap: 5, flexWrap: "wrap", justifyContent: "flex-end" }}>
                {agents.map((a, i) => (
                  <div
                    key={a.id}
                    style={{
                      width: 14,
                      height: 14,
                      background: a.online ? "var(--hp)" : "#5A5044",
                      border: "2px solid #fff",
                      animation: a.online ? "hud-blip 1.6s ease-in-out infinite" : "none",
                      animationDelay: `${i * 0.3}s`,
                    }}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      </header>

      <main style={{ maxWidth: 1180, margin: "0 auto", padding: "44px 32px 90px" }}>
        <section style={{ marginBottom: 48 }}>
          <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", marginBottom: 22 }}>
            <h1 className="pixel" style={{ fontSize: 16, margin: 0 }}>YOUR PARTY</h1>
            <span className="mono" style={{ fontSize: 12, color: "var(--muted)", fontWeight: 700 }}>
              {agents?.length ?? 0} UNIT{agents?.length === 1 ? "" : "S"}
            </span>
          </div>

          {isLoading && <p className="mono" style={{ color: "var(--muted)" }}>Loading…</p>}
          {agents && agents.length === 0 && <p className="mono" style={{ color: "var(--muted)" }}>No agents paired yet.</p>}

          {agents && agents.length > 0 && (
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))", gap: 18 }}>
              {agents.map((a) => (
                <div key={a.id} className="pixel-panel" style={{ padding: "20px 12px 16px", textAlign: "center" }}>
                  <AgentSprite agent={a} />
                  <Link
                    to={`/agents/${a.id}`}
                    className="mono"
                    style={{ display: "block", fontSize: 12, fontWeight: 700, margin: "12px 0 8px" }}
                  >
                    {a.id.slice(0, 8)}
                  </Link>
                  <HpBar online={a.online} />
                  <div className={a.status === "active" ? "status-tag status-tag-active" : "status-tag status-tag-revoked"}>
                    {a.status.toUpperCase()}
                  </div>
                  <div className="mono" style={{ fontSize: 10, color: "var(--muted)", marginTop: 6 }}>
                    {a.last_heartbeat_at ? new Date(a.last_heartbeat_at).toLocaleTimeString() : "never seen"}
                  </div>
                  {a.status === "active" && (
                    <button
                      type="button"
                      className="pixel-btn pixel-btn-danger pixel-btn-sm"
                      style={{ marginTop: 10 }}
                      onClick={() => setConfirmRevokeId(a.id)}
                    >
                      REVOKE
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </section>

        <section style={{ marginBottom: 48 }}>
          <div className="pixel-panel" style={{ padding: "28px 30px" }}>
            <div className="eyebrow">★ NEW RECRUIT</div>
            <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", gap: 26, flexWrap: "wrap" }}>
              {pairingToken ? (
                <PairingTokenCard token={pairingToken} issuedAt={pairingIssuedAt} />
              ) : (
                <p className="mono" style={{ color: "var(--muted)", margin: 0 }}>
                  Generate a token to pair a new agent.
                </p>
              )}
              <button type="button" className="pixel-btn" onClick={() => issueToken.mutate()} disabled={issueToken.isPending}>
                GENERATE TOKEN
              </button>
            </div>
            {issueToken.isError && <div className="alert-row">{(issueToken.error as Error).message}</div>}
          </div>
        </section>

        <section style={{ marginBottom: 48 }}>
          <div className="pixel-panel" style={{ padding: "28px 30px" }}>
            <div className="eyebrow">GITHUB</div>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 26, flexWrap: "wrap" }}>
              {githubLoading ? (
                <p className="mono" style={{ color: "var(--muted)", margin: 0 }}>Loading…</p>
              ) : github ? (
                <div>
                  <span className="badge" style={{ background: "#E4F9EE", color: "var(--hp-dark)", borderColor: "var(--hp-dark)" }}>
                    CONNECTED
                  </span>
                  <span className="mono" style={{ fontSize: 13, fontWeight: 700, marginLeft: 10 }}>
                    {github.github_login}
                  </span>
                  <div className="mono" style={{ fontSize: 11, color: "var(--muted)", marginTop: 6 }}>
                    {github.repos.length} repo{github.repos.length === 1 ? "" : "s"} available
                  </div>
                </div>
              ) : (
                <p className="mono" style={{ color: "var(--muted)", margin: 0 }}>
                  Connect a GitHub account to deploy containers straight from a repo.
                </p>
              )}
              {github ? (
                <button
                  type="button"
                  className="pixel-btn pixel-btn-ghost pixel-btn-sm"
                  onClick={() => disconnectGithub.mutate()}
                  disabled={disconnectGithub.isPending}
                >
                  DISCONNECT
                </button>
              ) : (
                <button type="button" className="pixel-btn" onClick={() => connectGithub.mutate()} disabled={connectGithub.isPending}>
                  CONNECT GITHUB
                </button>
              )}
            </div>
            {githubCallbackResult === "connected" && (
              <div className="alert-row alert-row-info">GitHub account connected.</div>
            )}
            {githubCallbackResult === "error" && (
              <div className="alert-row">Connecting to GitHub failed — try again.</div>
            )}
            {connectGithub.isError && <div className="alert-row">{(connectGithub.error as Error).message}</div>}
            {disconnectGithub.isError && <div className="alert-row">{(disconnectGithub.error as Error).message}</div>}
          </div>
        </section>

        <section>
          <div className="eyebrow">QUEST LOG</div>
          {securityEvents && securityEvents.length === 0 && (
            <p className="mono" style={{ color: "var(--muted)" }}>No activity yet.</p>
          )}
          {securityEvents && securityEvents.length > 0 && (
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {securityEvents.map((e, i) => (
                <div
                  key={i}
                  className="pixel-panel-sm"
                  style={{
                    padding: "12px 16px",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    ...(e.is_misuse_signal ? { background: "#FFF1F1" } : {}),
                  }}
                >
                  <span
                    className="mono"
                    style={{
                      fontSize: 12,
                      fontWeight: 700,
                      display: "flex",
                      alignItems: "center",
                      gap: 10,
                      color: e.is_misuse_signal ? "var(--danger-dark)" : undefined,
                    }}
                  >
                    {e.is_misuse_signal ? (
                      <img src={misuseIcon} alt="" style={{ width: 20, height: 20, objectFit: "contain" }} />
                    ) : (
                      <span
                        style={{ width: 12, height: 12, background: "var(--hp)", border: "2px solid var(--ink)", display: "inline-block" }}
                      />
                    )}
                    {EVENT_LABELS[e.event_type] ?? e.event_type}
                  </span>
                  <span className="mono" style={{ fontSize: 10.5, color: "var(--muted)" }}>
                    {new Date(e.created_at).toLocaleString()}
                  </span>
                </div>
              ))}
            </div>
          )}
        </section>
      </main>

      {confirmRevokeId && (
        <div
          onClick={(e) => {
            if (e.target === e.currentTarget) setConfirmRevokeId(null);
          }}
          role="dialog"
          aria-modal="true"
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.72)",
            zIndex: 1000,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: 24,
          }}
        >
          <div className="pixel-panel" style={{ width: "100%", maxWidth: 480, padding: "24px 26px", background: "var(--panel)" }}>
            <div className="eyebrow">REVOKE AGENT</div>
            <p className="mono" style={{ fontSize: 12.5, lineHeight: 1.7, margin: "10px 0 20px" }}>
              Revoke and permanently delete agent {confirmRevokeId.slice(0, 8)}? All of its stored state is removed
              from the database and its connection is cut. This can't be undone.
            </p>
            <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
              <button
                type="button"
                className="pixel-btn pixel-btn-ghost pixel-btn-sm"
                onClick={() => setConfirmRevokeId(null)}
                disabled={revoke.isPending}
              >
                CANCEL
              </button>
              <button
                type="button"
                className="pixel-btn pixel-btn-danger"
                onClick={() => revoke.mutate(confirmRevokeId)}
                disabled={revoke.isPending}
                autoFocus
              >
                {revoke.isPending ? "WORKING…" : "REVOKE & DELETE"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
