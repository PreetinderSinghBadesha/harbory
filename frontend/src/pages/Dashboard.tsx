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

  return (
    <div className="page">
      <header className="page-header">
        <h1>Harbory</h1>
        <button type="button" onClick={() => supabase.auth.signOut()}>
          Sign out
        </button>
      </header>

      <section>
        <h2>Pair a new agent</h2>
        <button type="button" onClick={() => issueToken.mutate()} disabled={issueToken.isPending}>
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
            <p>Run on the target VM:</p>
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
                  <td>{a.status}</td>
                  <td>{a.online ? "online" : "offline"}</td>
                  <td>{a.last_heartbeat_at ? new Date(a.last_heartbeat_at).toLocaleString() : "—"}</td>
                  <td>
                    {a.status === "active" && (
                      <button type="button" onClick={() => revoke.mutate(a.id)} disabled={revoke.isPending}>
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
    </div>
  );
}
