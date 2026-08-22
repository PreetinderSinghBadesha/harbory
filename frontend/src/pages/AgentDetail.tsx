import { useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../lib/api";

interface DesiredContainer {
  name: string;
  image: string;
  status: string;
}
interface ObservedContainer {
  name: string;
  image: string;
  status: string;
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

export function AgentDetail() {
  const { agentId } = useParams<{ agentId: string }>();
  const queryClient = useQueryClient();

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
  const [image, setImage] = useState("");
  const deployContainer = useMutation({
    mutationFn: () =>
      apiFetch<void>(`/agents/${agentId}/containers/${containerName}`, {
        method: "PUT",
        body: JSON.stringify({ image }),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["containers", agentId] });
      setContainerName("");
      setImage("");
    },
  });
  const removeContainer = useMutation({
    mutationFn: (name: string) => apiFetch<void>(`/agents/${agentId}/containers/${name}`, { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["containers", agentId] }),
  });

  const [routeName, setRouteName] = useState("");
  const [serverName, setServerName] = useState("");
  const [listenPort, setListenPort] = useState(80);
  const [upstreamHost, setUpstreamHost] = useState("127.0.0.1");
  const [upstreamPort, setUpstreamPort] = useState(8080);
  const deployRoute = useMutation({
    mutationFn: () =>
      apiFetch<void>(`/agents/${agentId}/proxy-routes/${routeName}`, {
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
    mutationFn: (name: string) => apiFetch<void>(`/agents/${agentId}/proxy-routes/${name}`, { method: "DELETE" }),
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

  return (
    <div className="page">
      <header className="page-header">
        <h1>Agent {agentId?.slice(0, 8)}</h1>
        <Link to="/dashboard" className="page-back">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
          Back to agents
        </Link>
      </header>

      <section>
        <h2>Containers</h2>
        <p className="hint">
          Changes take effect the next time this agent reports its state — up to one heartbeat
          interval, not instantly. See docs/reconciliation.md.
        </p>
        {containers.data && containers.data.desired.length > 0 && (
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Image</th>
                <th>Desired</th>
                <th>Observed</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {containers.data.desired.map((d) => {
                const observed = containers.data?.observed.find((o) => o.name === d.name);
                return (
                  <tr key={d.name}>
                    <td>{d.name}</td>
                    <td>{d.image}</td>
                    <td>{d.status}</td>
                    <td>{observed?.status ?? "—"}</td>
                    <td>
                      <button type="button" className="btn btn-danger btn-sm" onClick={() => removeContainer.mutate(d.name)}>
                        Remove
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
        <form onSubmit={handleDeployContainer} className="inline-form">
          <input placeholder="name" value={containerName} onChange={(e) => setContainerName(e.target.value)} required />
          <input placeholder="image (e.g. nginx:alpine)" value={image} onChange={(e) => setImage(e.target.value)} required />
          <button type="submit" className="btn btn-primary" disabled={deployContainer.isPending}>
            Deploy
          </button>
        </form>
        {deployContainer.isError && <p className="error">{(deployContainer.error as Error).message}</p>}
      </section>

      <section>
        <h2>Proxy routes</h2>
        {proxyRoutes.data?.error && <p className="error">Last apply error: {proxyRoutes.data.error}</p>}
        {proxyRoutes.data && proxyRoutes.data.desired.length > 0 && (
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Server name</th>
                <th>Listen</th>
                <th>Upstream</th>
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
                    <button type="button" className="btn btn-danger btn-sm" onClick={() => removeRoute.mutate(r.name)}>
                      Remove
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <form onSubmit={handleDeployRoute} className="inline-form">
          <input placeholder="name" value={routeName} onChange={(e) => setRouteName(e.target.value)} required />
          <input placeholder="server_name (optional)" value={serverName} onChange={(e) => setServerName(e.target.value)} />
          <input
            type="number"
            placeholder="listen port"
            value={listenPort}
            onChange={(e) => setListenPort(Number(e.target.value))}
            required
          />
          <input placeholder="upstream host" value={upstreamHost} onChange={(e) => setUpstreamHost(e.target.value)} required />
          <input
            type="number"
            placeholder="upstream port"
            value={upstreamPort}
            onChange={(e) => setUpstreamPort(Number(e.target.value))}
            required
          />
          <button type="submit" className="btn btn-primary" disabled={deployRoute.isPending}>
            Deploy route
          </button>
        </form>
        {deployRoute.isError && <p className="error">{(deployRoute.error as Error).message}</p>}
      </section>
    </div>
  );
}
