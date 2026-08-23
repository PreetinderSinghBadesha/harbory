import { useState, useCallback, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../lib/api";
import { situationFor, spriteFor } from "../lib/agentSprite";
import { fetchGitHubConnection, repoUrlFor } from "../lib/github";
import "../styles/GameHud.css";

interface AgentSummary {
  id: string;
  status: string;
  online: boolean;
  last_heartbeat_at: string | null;
}

interface DesiredContainer {
  name: string;
  image: string;
  status: string;
  source: { repo_url: string; git_ref: string; dockerfile_path: string } | null;
}
interface ObservedContainer {
  name: string;
  image: string;
  status: string;
  error?: string | null;
}
interface ContainersResponse {
  desired: DesiredContainer[];
  observed: ObservedContainer[];
}

interface ProxyRoute {
  name: string;
  server_name: string;
  listen_port: number;
  path_prefix: string;
  upstream_host: string;
  upstream_port: number;
}
interface ProxyRoutesResponse {
  desired: ProxyRoute[];
  applied_hash: string | null;
  error: string | null;
}

interface ContainerLogsDto {
  logs: string;
  error: string;
}

interface LogsModalState {
  containerName: string;
  agentId: string;
}

function LogsModal({ containerName, agentId, onClose }: LogsModalState & { onClose: () => void }) {
  const [tail, setTail] = useState(100);

  const { data, isFetching, error, refetch } = useQuery({
    queryKey: ["container-logs", agentId, containerName, tail],
    queryFn: () => apiFetch<ContainerLogsDto>(`/agents/${agentId}/containers/${encodeURIComponent(containerName)}/logs?tail=${tail}`),
    retry: false,
    staleTime: 0,
  });

  const handleBackdropClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) onClose();
  }, [onClose]);

  return (
    <div
      onClick={handleBackdropClick}
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
      <div
        className="pixel-panel"
        style={{
          width: "100%",
          maxWidth: 820,
          maxHeight: "80vh",
          display: "flex",
          flexDirection: "column",
          background: "#0F0E0C",
          color: "#E8DCC8",
          overflow: "hidden",
        }}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "12px 16px",
            borderBottom: "3px solid #2A2520",
            flexShrink: 0,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span className="pixel" style={{ fontSize: 11, color: "var(--gold)" }}>LOGS</span>
            <span className="mono" style={{ fontSize: 12, color: "#888", fontWeight: 600 }}>{containerName}</span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <select
              className="mono"
              value={tail}
              onChange={(e) => setTail(Number(e.target.value))}
              style={{
                background: "#1A1814",
                color: "#E8DCC8",
                border: "2px solid #3A342C",
                padding: "3px 6px",
                fontSize: 11,
                fontWeight: 700,
                cursor: "pointer",
              }}
            >
              <option value={50}>50 lines</option>
              <option value={100}>100 lines</option>
              <option value={200}>200 lines</option>
              <option value={500}>500 lines</option>
            </select>
            <button
              type="button"
              className="pixel-btn pixel-btn-ghost pixel-btn-sm"
              onClick={() => refetch()}
              disabled={isFetching}
              style={{ fontSize: 10 }}
            >
              {isFetching ? "…" : "↺ REFRESH"}
            </button>
            <button
              type="button"
              className="pixel-btn pixel-btn-ghost pixel-btn-sm"
              onClick={onClose}
              style={{ fontSize: 10 }}
            >
              ✕ CLOSE
            </button>
          </div>
        </div>

        {/* Body */}
        <div style={{ flex: 1, overflow: "auto", padding: 16, minHeight: 0 }}>
          {isFetching && !data && (
            <p className="mono" style={{ color: "#888", fontSize: 12 }}>Fetching logs…</p>
          )}
          {error && (
            <div className="alert-row" style={{ fontSize: 12 }}>
              {(error as { status?: number }).status === 503 || (error as Error).message?.includes("503")
                ? "Agent is offline — logs not available."
                : (error as { status?: number }).status === 504 || (error as Error).message?.includes("504")
                  ? "Timed out waiting for the agent (5 s). Try again."
                  : (error as Error).message}
            </div>
          )}
          {data?.error && !data.logs && (
            <div className="alert-row" style={{ fontSize: 12 }}>{data.error}</div>
          )}
          {data?.logs && (
            <pre
              style={{
                margin: 0,
                fontFamily: "'JetBrains Mono', 'Fira Mono', monospace",
                fontSize: 12,
                lineHeight: 1.6,
                color: "#C8D8B8",
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
              }}
            >
              {data.logs}
            </pre>
          )}
          {data && !data.logs && !data.error && !isFetching && (
            <p className="mono" style={{ color: "#888", fontSize: 12 }}>No log output yet.</p>
          )}
        </div>
      </div>
    </div>
  );
}

function HpBar({ online }: { online: boolean }) {
  return (
    <div className="hp-bar" style={{ justifyContent: "flex-start", marginBottom: 6 }}>
      {Array.from({ length: 6 }).map((_, i) => (
        <span key={i} className={online ? "hp-pip hp-pip-on hp-pip-big" : "hp-pip hp-pip-big"} />
      ))}
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const s = status.toLowerCase();
  let bg = "#E4F9EE";
  let color = "var(--hp-dark)";
  let border = "var(--hp-dark)";

  if (s === "error" || s === "dead" || s === "failed") {
    bg = "#FDE8E8";
    color = "var(--clay)";
    border = "var(--clay)";
  } else if (s === "absent" || s === "removing" || s === "removed" || s === "stopped" || s === "exited") {
    bg = "#F3F0EA";
    color = "#8A7E72";
    border = "#8A7E72";
  }

  return (
    <span className="badge" style={{ background: bg, color, borderColor: border }}>
      {status.toUpperCase()}
    </span>
  );
}

export function AgentDetail() {
  const { agentId } = useParams<{ agentId: string }>();
  const queryClient = useQueryClient();

  const [logsModal, setLogsModal] = useState<{ containerName: string } | null>(null);

  // No single-agent endpoint exists — the list is the only source for this
  // agent's own status/online/last-heartbeat, so this page shares the same
  // query (and cache) the dashboard uses rather than inventing a new call.
  const { data: agents } = useQuery({
    queryKey: ["agents"],
    queryFn: () => apiFetch<AgentSummary[]>("/agents"),
    refetchInterval: 5000,
  });
  const agent = agents?.find((a) => a.id === agentId);

  const revoke = useMutation({
    mutationFn: () => apiFetch<void>(`/agents/${agentId}/revoke`, { method: "POST" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["agents"] }),
  });

  const containers = useQuery({
    queryKey: ["containers", agentId],
    queryFn: () => apiFetch<ContainersResponse>(`/agents/${agentId}/containers`),
    refetchInterval: 5000,
  });
  const proxyRoutes = useQuery({
    queryKey: ["proxy-routes", agentId],
    queryFn: () => apiFetch<ProxyRoutesResponse>(`/agents/${agentId}/proxy-routes`),
    refetchInterval: 5000,
  });

  const [containerName, setContainerName] = useState("");
  const [deploySource, setDeploySource] = useState<"image" | "github">("image");
  const [image, setImage] = useState("");
  const [selectedRepo, setSelectedRepo] = useState("");
  const [gitRef, setGitRef] = useState("");
  const [dockerfilePath, setDockerfilePath] = useState("");

  const github = useQuery({
    queryKey: ["github-connection"],
    queryFn: fetchGitHubConnection,
  });

  const deployContainer = useMutation({
    mutationFn: () =>
      apiFetch<void>(`/agents/${agentId}/containers/${encodeURIComponent(containerName)}`, {
        method: "PUT",
        body: JSON.stringify(
          deploySource === "github"
            ? { source: { repo_url: repoUrlFor(selectedRepo), git_ref: gitRef, dockerfile_path: dockerfilePath } }
            : { image },
        ),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["containers", agentId] });
      setContainerName("");
      setImage("");
      setSelectedRepo("");
      setGitRef("");
      setDockerfilePath("");
    },
  });
  const removeContainer = useMutation({
    mutationFn: (name: string) =>
      apiFetch<void>(`/agents/${agentId}/containers/${encodeURIComponent(name)}`, { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["containers", agentId] }),
  });

  const [routeName, setRouteName] = useState("");
  const [serverName, setServerName] = useState("");
  const [listenPort, setListenPort] = useState(80);
  const [upstreamHost, setUpstreamHost] = useState("127.0.0.1");
  const [upstreamPort, setUpstreamPort] = useState(8080);
  const deployRoute = useMutation({
    mutationFn: () =>
      apiFetch<void>(`/agents/${agentId}/proxy-routes/${encodeURIComponent(routeName)}`, {
        method: "PUT",
        body: JSON.stringify({
          server_name: serverName,
          listen_port: listenPort,
          upstream_host: upstreamHost,
          upstream_port: upstreamPort,
        }),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["proxy-routes", agentId] });
      setRouteName("");
    },
  });
  const removeRoute = useMutation({
    mutationFn: (name: string) =>
      apiFetch<void>(`/agents/${agentId}/proxy-routes/${encodeURIComponent(name)}`, { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["proxy-routes", agentId] }),
  });

  function handleDeployContainer(e: FormEvent) {
    e.preventDefault();
    deployContainer.mutate();
  }
  function handleDeployRoute(e: FormEvent) {
    e.preventDefault();
    deployRoute.mutate();
  }

  const sprite = agentId ? spriteFor(agentId) : null;
  const situation = agent ? situationFor(agent) : "offline";

  return (
    <div className="game-hud">
      <header style={{ background: "var(--panel)", borderBottom: "4px solid var(--ink)", padding: "20px 32px" }}>
        <div style={{ maxWidth: 1180, margin: "0 auto", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
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
          <Link to="/dashboard" className="pixel-btn pixel-btn-ghost pixel-btn-sm">
            ← BACK
          </Link>
        </div>
      </header>

      <main style={{ maxWidth: 1180, margin: "0 auto", padding: "44px 32px 90px" }}>
        <section>
          <div className="pixel-panel" style={{ padding: 0, overflow: "hidden" }}>
            <div
              style={{
                background: "linear-gradient(180deg, #DCEBFF 0%, var(--bg) 100%)",
                padding: "28px 30px 20px",
                borderBottom: "4px solid var(--ink)",
                boxShadow: "inset 0 3px 0 rgba(255,255,255,0.5)",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                flexWrap: "wrap",
                gap: 16,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 20 }}>
                <div>
                  <div className={`sprite-stage sprite-stage-big sprite-${situation}`}>
                    {sprite &&
                      (situation === "online" ? (
                        <>
                          <img className="f1" src={sprite.drive1} alt="" />
                          <img className="f2" src={sprite.drive2} alt="" />
                        </>
                      ) : (
                        <img
                          src={sprite.hurt}
                          alt=""
                          style={{ position: "relative", width: "100%", height: "100%", objectFit: "contain" }}
                        />
                      ))}
                  </div>
                  <div className="ground" style={{ width: 130, marginTop: 8 }} />
                </div>
                <div>
                  <div className="eyebrow" style={{ marginBottom: 6 }}>AGENT</div>
                  <div className="mono" style={{ fontSize: 24, fontWeight: 700, marginBottom: 10 }}>
                    {agentId?.slice(0, 8)}
                  </div>
                  {agent && <HpBar online={agent.online} />}
                  <span className="mono" style={{ fontSize: 11, color: "var(--muted)", fontWeight: 600 }}>
                    {situation === "revoked"
                      ? "revoked"
                      : agent?.online
                        ? "online"
                        : agent?.last_heartbeat_at
                          ? `last seen ${new Date(agent.last_heartbeat_at).toLocaleString()}`
                          : "never seen"}
                  </span>
                </div>
              </div>
              {agent?.status === "active" && (
                <button
                  type="button"
                  className="pixel-btn pixel-btn-danger"
                  onClick={() => revoke.mutate()}
                  disabled={revoke.isPending}
                >
                  REVOKE
                </button>
              )}
            </div>

            <div style={{ padding: "26px 30px" }}>
              <div style={{ marginBottom: 24 }}>
                <div className="eyebrow" style={{ marginBottom: 10 }}>CONTAINERS</div>
                <p className="mono" style={{ fontSize: 11, color: "var(--muted)", marginTop: 0 }}>
                  Changes take effect the next time this agent reports its state — up to one heartbeat interval, not
                  instantly.
                </p>
                {(() => {
                  const activeContainers = containers.data?.desired.filter((d) => {
                    if (d.status !== "absent") return true;
                    const observed = containers.data?.observed.find((o) => o.name === d.name);
                    return observed && observed.status !== "removed";
                  });
                  if (!activeContainers || activeContainers.length === 0) return null;
                  return (
                    <table>
                      <thead>
                        <tr>
                          <th>NAME</th>
                          <th>IMAGE / SOURCE</th>
                          <th>DESIRED</th>
                          <th>OBSERVED</th>
                          <th />
                        </tr>
                      </thead>
                      <tbody>
                        {activeContainers.map((d) => {
                          const observed = containers.data?.observed.find((o) => o.name === d.name);
                          const isAbsent = d.status === "absent";
                          return (
                            <tr key={d.name}>
                              <td>{d.name}</td>
                              <td>
                                {d.source ? (
                                  <>
                                    <span className="badge" style={{ background: "#E9F4FF", color: "var(--blue)", borderColor: "var(--blue)", marginRight: 6 }}>
                                      GITHUB
                                    </span>
                                    {d.source.repo_url.replace(/^https:\/\/github\.com\//, "").replace(/\.git$/, "")}
                                    {d.source.git_ref && `@${d.source.git_ref}`}
                                  </>
                                ) : (
                                  d.image
                                )}
                              </td>
                              <td>
                                <StatusBadge status={isAbsent ? "REMOVING" : d.status} />
                              </td>
                              <td>
                                {observed ? (
                                  <div>
                                    <StatusBadge status={observed.status} />
                                    {observed.error && (
                                      <div className="mono" style={{ fontSize: 10.5, color: "var(--clay)", marginTop: 4, maxWidth: 300, wordBreak: "break-word" }}>
                                        ⚠ {observed.error}
                                      </div>
                                    )}
                                  </div>
                                ) : (
                                  "—"
                                )}
                              </td>
                              <td>
                                <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                                  <button
                                    type="button"
                                    className="pixel-btn pixel-btn-ghost pixel-btn-sm"
                                    onClick={() => setLogsModal({ containerName: d.name })}
                                  >
                                    LOGS
                                  </button>
                                  <button
                                    type="button"
                                    className="pixel-btn pixel-btn-danger pixel-btn-sm"
                                    onClick={() => removeContainer.mutate(d.name)}
                                    disabled={removeContainer.isPending || isAbsent}
                                  >
                                    {isAbsent ? "REMOVING…" : "REMOVE"}
                                  </button>
                                </div>
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  );
                })()}
                <div style={{ display: "flex", gap: 8, marginTop: 14, marginBottom: 10 }}>
                  <button
                    type="button"
                    className={deploySource === "image" ? "pixel-btn pixel-btn-sm" : "pixel-btn pixel-btn-ghost pixel-btn-sm"}
                    onClick={() => setDeploySource("image")}
                  >
                    IMAGE
                  </button>
                  <button
                    type="button"
                    className={deploySource === "github" ? "pixel-btn pixel-btn-sm" : "pixel-btn pixel-btn-ghost pixel-btn-sm"}
                    onClick={() => setDeploySource("github")}
                  >
                    FROM GITHUB REPO
                  </button>
                </div>
                <form onSubmit={handleDeployContainer} className="inline-form">
                  <input placeholder="name" value={containerName} onChange={(e) => setContainerName(e.target.value)} required />
                  {deploySource === "image" ? (
                    <input
                      placeholder="image (e.g. nginx:alpine)"
                      value={image}
                      onChange={(e) => setImage(e.target.value)}
                      required
                    />
                  ) : github.data && github.data.repos.length > 0 ? (
                    <>
                      <select value={selectedRepo} onChange={(e) => setSelectedRepo(e.target.value)} required>
                        <option value="" disabled>
                          select a repo…
                        </option>
                        {github.data.repos.map((r) => (
                          <option key={r.full_name} value={r.full_name}>
                            {r.full_name}
                            {r.private ? " (private)" : ""}
                          </option>
                        ))}
                      </select>
                      <input
                        placeholder="branch (default: repo's default branch)"
                        value={gitRef}
                        onChange={(e) => setGitRef(e.target.value)}
                      />
                      <input
                        placeholder="Dockerfile path (default: Dockerfile)"
                        value={dockerfilePath}
                        onChange={(e) => setDockerfilePath(e.target.value)}
                      />
                    </>
                  ) : (
                    <p className="mono" style={{ fontSize: 11.5, color: "var(--muted)", margin: 0 }}>
                      {github.isLoading ? "Loading repos…" : "Connect a GitHub account from the dashboard first."}
                    </p>
                  )}
                  <button
                    type="submit"
                    className="pixel-btn pixel-btn-sm"
                    disabled={deployContainer.isPending || (deploySource === "github" && !github.data?.repos.length)}
                  >
                    DEPLOY
                  </button>
                </form>
                {deployContainer.isError && <div className="alert-row">{(deployContainer.error as Error).message}</div>}
              </div>

              <div>
                <div className="eyebrow" style={{ marginBottom: 10 }}>PROXY ROUTES</div>
                {proxyRoutes.data?.error && <div className="alert-row">Last apply error: {proxyRoutes.data.error}</div>}
                {proxyRoutes.data && proxyRoutes.data.desired.length > 0 && (
                  <table>
                    <thead>
                      <tr>
                        <th>NAME</th>
                        <th>SERVER NAME</th>
                        <th>LISTEN</th>
                        <th>UPSTREAM</th>
                        <th />
                      </tr>
                    </thead>
                    <tbody>
                      {proxyRoutes.data.desired.map((r) => (
                        <tr key={r.name}>
                          <td>{r.name}</td>
                          <td>{r.server_name || "(catch-all)"}</td>
                          <td>{r.listen_port}</td>
                          <td>
                            {r.upstream_host}:{r.upstream_port}
                          </td>
                          <td>
                            <button
                              type="button"
                              className="pixel-btn pixel-btn-danger pixel-btn-sm"
                              onClick={() => removeRoute.mutate(r.name)}
                            >
                              REMOVE
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
                <form onSubmit={handleDeployRoute} className="inline-form">
                  <input placeholder="name" value={routeName} onChange={(e) => setRouteName(e.target.value)} required />
                  <input
                    placeholder="server_name (optional)"
                    value={serverName}
                    onChange={(e) => setServerName(e.target.value)}
                  />
                  <input
                    type="number"
                    placeholder="listen port"
                    value={listenPort}
                    onChange={(e) => setListenPort(Number(e.target.value))}
                    required
                  />
                  <input
                    placeholder="upstream host"
                    value={upstreamHost}
                    onChange={(e) => setUpstreamHost(e.target.value)}
                    required
                  />
                  <input
                    type="number"
                    placeholder="upstream port"
                    value={upstreamPort}
                    onChange={(e) => setUpstreamPort(Number(e.target.value))}
                    required
                  />
                  <button type="submit" className="pixel-btn pixel-btn-sm" disabled={deployRoute.isPending}>
                    DEPLOY ROUTE
                  </button>
                </form>
                {deployRoute.isError && <div className="alert-row">{(deployRoute.error as Error).message}</div>}
              </div>
            </div>
          </div>
        </section>
      </main>

      {logsModal && agentId && (
        <LogsModal
          agentId={agentId}
          containerName={logsModal.containerName}
          onClose={() => setLogsModal(null)}
        />
      )}
    </div>
  );
}
