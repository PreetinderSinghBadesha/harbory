import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { apiFetch } from "../lib/api";
import { supabase } from "../lib/supabase";

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
};

export function Dashboard() {
  const queryClient = useQueryClient();

  const { data: agents, isLoading } = useQuery({
    queryKey: ["agents"],
    queryFn: () => apiFetch<AgentSummary[]>("/agents"),
    refetchInterval: 5000, // matches the ~10s heartbeat cadence closely enough for a dashboard
  });

  const [pairingToken, setPairingToken] = useState<PairingToken | null>(null);
  const issueToken = useMutation({
    mutationFn: () => apiFetch<PairingToken>("/pairing-tokens", { method: "POST", body: JSON.stringify({}) }),
    onSuccess: setPairingToken,
  });

  const revoke = useMutation({
    mutationFn: (agentId: string) => apiFetch<void>(`/agents/${agentId}/revoke`, { method: "POST" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["agents"] }),
  });

  const { data: securityEvents } = useQuery({
    queryKey: ["security-events"],
    queryFn: () => apiFetch<SecurityEvent[]>("/security-events"),
    refetchInterval: 10000,
  });

  return (
    <div className="page">
      <header className="page-header">
        <Link to="/" className="page-brand">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--h-accent)" strokeWidth="1.6" strokeLinejoin="round" aria-hidden="true">
            <path d="M12 2.5 L21 7.5 V16.5 L12 21.5 L3 16.5 V7.5 Z" />
            <path d="M3 7.5 L12 12.5 L21 7.5" />
            <path d="M12 12.5 V21.5" />
          </svg>
          Harbory
        </Link>
        <button type="button" className="btn btn-ghost" onClick={() => supabase.auth.signOut()}>
          Sign out
        </button>
      </header>

      <section>
        <h2>Pair a new agent</h2>
        <button type="button" className="btn btn-primary" onClick={() => issueToken.mutate()} disabled={issueToken.isPending}>
          Generate pairing token
        </button>
        {issueToken.isError && <p className="error">{(issueToken.error as Error).message}</p>}
        {pairingToken && (
          <div className="pairing-token">
            {/* Single-use, shown once — matches the security model (§3): a
               pairing token is displayed here and never stored anywhere
               the dashboard can retrieve again. */}
            <p>Token (expires {new Date(pairingToken.expires_at).toLocaleString()}), single-use — copy it now:</p>
            <code>{pairingToken.token}</code>
            <p>On the target VM, first time only (needs a Rust toolchain — installs just the agent binary, not this repo):</p>
            <pre>cargo install --git https://github.com/PreetinderSinghBadesha/harbory.git harbory-agent</pre>
            <p>Then run:</p>
            <pre>harbory-agent {pairingToken.token}</pre>
          </div>
        )}
      </section>

      <section>
        <h2>Agents</h2>
        {isLoading && <p className="page-status">Loading…</p>}
        {agents && agents.length === 0 && <p className="page-status">No agents paired yet.</p>}
        {agents && agents.length > 0 && (
          <table>
            <thead>
              <tr>
                <th>Agent</th>
                <th>Status</th>
                <th>Online</th>
                <th>Last heartbeat</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {agents.map((a) => (
                <tr key={a.id}>
                  <td>
                    <Link to={`/agents/${a.id}`}>{a.id.slice(0, 8)}</Link>
                  </td>
                  <td>
                    <span className={a.status === "active" ? "badge badge-ok" : "badge badge-warn"}>{a.status}</span>
                  </td>
                  <td>
                    <span className={a.online ? "badge badge-ok" : "badge badge-off"}>
                      {a.online ? "online" : "offline"}
                    </span>
                  </td>
                  <td>{a.last_heartbeat_at ? new Date(a.last_heartbeat_at).toLocaleString() : "—"}</td>
                  <td>
                    {a.status === "active" && (
                      <button
                        type="button"
                        className="btn btn-danger btn-sm"
                        onClick={() => revoke.mutate(a.id)}
                        disabled={revoke.isPending}
                      >
                        Revoke
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section>
        <h2>Activity</h2>
        {securityEvents && securityEvents.length === 0 && <p className="page-status">No activity yet.</p>}
        {securityEvents && securityEvents.length > 0 && (
          <ul className="activity-list">
            {securityEvents.map((e, i) => (
              <li key={i} className={e.is_misuse_signal ? "activity-item activity-item-warn" : "activity-item"}>
                <span>{EVENT_LABELS[e.event_type] ?? e.event_type}</span>
                <span className="activity-time">{new Date(e.created_at).toLocaleString()}</span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
