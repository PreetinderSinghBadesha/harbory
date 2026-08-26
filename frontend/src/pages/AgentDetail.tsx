import { useState, useEffect, useCallback, type FormEvent, type ReactNode } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
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
  env?: string[];
  ports?: { host_port: number; container_port: number }[];
  command?: string[];
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

interface DesiredComposeStack {
  name: string;
  status: string;
  source: { repo_url: string; git_ref: string; dockerfile_path: string } | null;
  compose_file_path: string;
  env: string[];
}
interface ObservedComposeStack {
  name: string;
  status: string;
  error?: string | null;
}
interface ComposeStacksResponse {
  desired: DesiredComposeStack[];
  observed: ObservedComposeStack[];
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

interface AgentImage {
  id: string;
  repo_tags: string[];
  size_bytes: number;
  created_at: number;
  in_use: boolean;
}
interface ImagesDto {
  images: AgentImage[];
  error: string;
}

type DeployTab = "container" | "compose";

type PendingAction =
  | { kind: "container"; name: string }
  | { kind: "compose"; name: string }
  | { kind: "route"; name: string }
  | { kind: "image"; id: string; label: string }
  | { kind: "revoke" };

function confirmCopy(action: PendingAction): { title: string; message: string; confirmLabel: string } {
  switch (action.kind) {
    case "container":
      return {
        title: "REMOVE CONTAINER",
        message: `Remove container "${action.name}"? It will be stopped and deleted from the agent on its next heartbeat.`,
        confirmLabel: "REMOVE",
      };
    case "compose":
      return {
        title: "REMOVE COMPOSE STACK",
        message: `Remove compose stack "${action.name}"? All of its containers will be stopped and removed on the agent's next heartbeat.`,
        confirmLabel: "REMOVE",
      };
    case "route":
      return {
        title: "REMOVE PROXY ROUTE",
        message: `Remove proxy route "${action.name}"? Traffic to ${action.name} will stop being proxied after the next heartbeat.`,
        confirmLabel: "REMOVE",
      };
    case "image":
      return {
        title: "DELETE IMAGE",
        message: `Delete image ${action.label}? It will be removed from this host's Docker storage and re-downloaded on the next deploy that needs it.`,
        confirmLabel: "DELETE",
      };
    case "revoke":
      return {
        title: "REVOKE AGENT",
        message:
          "Revoke and permanently delete this agent? All of its stored state — containers, stacks, proxy routes — is removed from the database and its live connection is cut. The host itself keeps running; pair a new agent to manage it again. This can't be undone.",
        confirmLabel: "REVOKE & DELETE",
      };
  }
}

function ModalBackdrop({ onClose, children, labelledBy }: { onClose: () => void; children: ReactNode; labelledBy?: string }) {
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (e.target === e.currentTarget) onClose();
    },
    [onClose],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      onClick={handleBackdropClick}
      role="dialog"
      aria-modal="true"
      aria-labelledby={labelledBy}
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
      {children}
    </div>
  );
}

function ConfirmDialog({
  action,
  busy,
  onConfirm,
  onCancel,
}: {
  action: PendingAction;
  busy: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const copy = confirmCopy(action);
  return (
    <ModalBackdrop onClose={onCancel} labelledBy="confirm-title">
      <div className="pixel-panel" style={{ width: "100%", maxWidth: 480, padding: "24px 26px", background: "var(--panel)" }}>
        <div id="confirm-title" className="eyebrow">{copy.title}</div>
        <p className="mono" style={{ fontSize: 12.5, lineHeight: 1.7, margin: "10px 0 20px", wordBreak: "break-word" }}>
          {copy.message}
        </p>
        <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
          <button type="button" className="pixel-btn pixel-btn-ghost pixel-btn-sm" onClick={onCancel} disabled={busy}>
            CANCEL
          </button>
          <button
            type="button"
            className={action.kind === "revoke" ? "pixel-btn pixel-btn-danger" : "pixel-btn pixel-btn-danger pixel-btn-sm"}
            onClick={onConfirm}
            disabled={busy}
            autoFocus
          >
            {busy ? "WORKING…" : copy.confirmLabel}
          </button>
        </div>
      </div>
    </ModalBackdrop>
  );
}

function LogsModal({ containerName, agentId, onClose }: { containerName: string; agentId: string; onClose: () => void }) {
  const [tail, setTail] = useState(100);

  const { data, isFetching, error, refetch } = useQuery({
    queryKey: ["container-logs", agentId, containerName, tail],
    queryFn: () => apiFetch<ContainerLogsDto>(`/agents/${agentId}/containers/${encodeURIComponent(containerName)}/logs?tail=${tail}`),
    retry: false,
    staleTime: 0,
  });

  return (
    <ModalBackdrop onClose={onClose} labelledBy="logs-title">
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
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "12px 16px",
            borderBottom: "3px solid #2A2520",
            flexShrink: 0,
            flexWrap: "wrap",
            gap: 8,
          }}
        >
          <div id="logs-title" style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span className="pixel" style={{ fontSize: 11, color: "var(--gold)" }}>LOGS</span>
            <span className="mono" style={{ fontSize: 12, color: "#888", fontWeight: 600 }}>{containerName}</span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <select
              className="mono"
              value={tail}
              onChange={(e) => setTail(Number(e.target.value))}
              aria-label="Number of log lines"
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
              autoFocus
              style={{ fontSize: 10 }}
            >
              ✕ CLOSE
            </button>
          </div>
        </div>

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
    </ModalBackdrop>
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

function RepoTag({ repoUrl, gitRef }: { repoUrl: string; gitRef?: string }) {
  const label = repoUrl.replace(/^https:\/\/github\.com\//, "").replace(/\.git$/, "");
  return (
    <>
      <span className="badge" style={{ background: "#E9F4FF", color: "var(--blue)", borderColor: "var(--blue)", marginRight: 6 }}>
        GITHUB
      </span>
      {label}
      {gitRef ? `@${gitRef}` : ""}
    </>
  );
}

function SectionPanel({
  title,
  count,
  action,
  children,
}: {
  title: string;
  count?: number;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="pixel-panel" style={{ padding: "22px 28px 26px", marginBottom: 26 }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
        <div className="eyebrow" style={{ marginBottom: 0, display: "flex", alignItems: "center", gap: 10 }}>
          {title}
          {typeof count === "number" && (
            <span className="badge">{count}</span>
          )}
        </div>
        {action}
      </div>
      <div style={{ marginTop: 16 }}>{children}</div>
    </div>
  );
}

function CollapsibleForm({ title, open, onCancel, children }: { title: string; open: boolean; onCancel: () => void; children: ReactNode }) {
  if (!open) return null;
  return (
    <div className="form-box">
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 10, marginBottom: 14 }}>
        <span className="eyebrow" style={{ marginBottom: 0 }}>{title}</span>
        <button type="button" className="pixel-btn pixel-btn-ghost pixel-btn-sm" onClick={onCancel}>
          CANCEL
        </button>
      </div>
      {children}
    </div>
  );
}

function EmptyHint({ children }: { children: ReactNode }) {
  return (
    <p className="mono" style={{ fontSize: 12, color: "var(--muted)", margin: "4px 0 0" }}>
      {children}
    </p>
  );
}

function formatSize(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

function imageLabel(img: AgentImage): string {
  return img.repo_tags[0] ?? img.id.replace(/^sha256:/, "").slice(0, 12);
}

function Field({ label, optional, children }: { label: string; optional?: boolean; children: ReactNode }) {
  return (
    <label style={{ display: "flex", flexDirection: "column", gap: 5, flex: "1 1 150px", minWidth: 0 }}>
      <span className="field-label">
        {label}
        {optional ? " · OPTIONAL" : ""}
      </span>
      {children}
    </label>
  );
}

function EnvVarsEditor({
  vars,
  onChange,
}: {
  vars: { key: string; value: string }[];
  onChange: (next: { key: string; value: string }[]) => void;
}) {
  return (
    <div>
      <div className="field-label" style={{ marginBottom: 8 }}>ENVIRONMENT VARIABLES</div>
      {vars.length === 0 && (
        <p className="mono" style={{ fontSize: 11.5, color: "var(--muted)", margin: "0 0 8px" }}>
          None — add variables your container expects.
        </p>
      )}
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {vars.map((env, i) => (
          <div key={i} style={{ display: "flex", gap: 6, alignItems: "center" }}>
            <input
              placeholder="VAR"
              value={env.key}
              onChange={(e) => {
                const next = [...vars];
                next[i].key = e.target.value;
                onChange(next);
              }}
              style={{ width: 140 }}
            />
            <span className="mono" style={{ color: "#888" }}>=</span>
            <input
              placeholder="value"
              value={env.value}
              onChange={(e) => {
                const next = [...vars];
                next[i].value = e.target.value;
                onChange(next);
              }}
              style={{ width: 180 }}
            />
            <button
              type="button"
              className="pixel-btn pixel-btn-ghost pixel-btn-sm"
              onClick={() => {
                const next = [...vars];
                next.splice(i, 1);
                onChange(next);
              }}
              aria-label={`Remove variable ${env.key || i + 1}`}
              style={{ fontSize: 10, padding: "2px 6px" }}
            >
              ✕
            </button>
          </div>
        ))}
      </div>
      <button
        type="button"
        className="pixel-btn pixel-btn-ghost pixel-btn-sm"
        onClick={() => onChange([...vars, { key: "", value: "" }])}
        style={{ marginTop: 8 }}
      >
        + ADD VARIABLE
      </button>
    </div>
  );
}

export function AgentDetail() {
  const { agentId } = useParams<{ agentId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const [logsModal, setLogsModal] = useState<{ containerName: string } | null>(null);
  const [confirmAction, setConfirmAction] = useState<PendingAction | null>(null);
  const [notice, setNotice] = useState<{ text: string; tone: "info" | "error" } | null>(null);

  useEffect(() => {
    if (!notice) return;
    const id = window.setTimeout(() => setNotice(null), 4000);
    return () => window.clearTimeout(id);
  }, [notice]);

  // No single-agent endpoint exists — the list is the only source for this
  // agent's own status/online/last-heartbeat, so this page shares the same
  // query (and cache) the dashboard uses rather than inventing a new call.
  const { data: agents } = useQuery({
    queryKey: ["agents"],
    queryFn: () => apiFetch<AgentSummary[]>("/agents"),
    refetchInterval: 5000,
  });
  const agent = agents?.find((a) => a.id === agentId);
  const situation = agent ? situationFor(agent) : "offline";
  const isRevoked = situation === "revoked";

  // "Revoke" deletes the agent outright — the DB row and all scoped state
  // go with it (see DELETE /agents/:id), and its live connection is kicked.
  const revoke = useMutation({
    mutationFn: () => apiFetch<void>(`/agents/${agentId}`, { method: "DELETE" }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["agents"] });
      navigate("/dashboard");
    },
    onError: (error) => {
      setNotice({ text: `Couldn't revoke agent: ${(error as Error).message}`, tone: "error" });
      setConfirmAction(null);
    },
  });

  const containers = useQuery({
    queryKey: ["containers", agentId],
    queryFn: () => apiFetch<ContainersResponse>(`/agents/${agentId}/containers`),
    refetchInterval: 5000,
  });
  const activeContainers = (containers.data?.desired ?? []).filter((d) => {
    if (d.status !== "absent") return true;
    const observed = containers.data?.observed.find((o) => o.name === d.name);
    return observed !== undefined && observed.status !== "removed";
  });

  const proxyRoutes = useQuery({
    queryKey: ["proxy-routes", agentId],
    queryFn: () => apiFetch<ProxyRoutesResponse>(`/agents/${agentId}/proxy-routes`),
    refetchInterval: 5000,
  });

  const [deployTab, setDeployTab] = useState<DeployTab>("container");
  const [showContainerForm, setShowContainerForm] = useState(false);
  const [showComposeForm, setShowComposeForm] = useState(false);
  const [showRouteForm, setShowRouteForm] = useState(false);

  const [containerName, setContainerName] = useState("");
  const [deploySource, setDeploySource] = useState<"image" | "github">("image");
  const [image, setImage] = useState("");
  const [selectedRepo, setSelectedRepo] = useState("");
  const [gitRef, setGitRef] = useState("");
  const [dockerfilePath, setDockerfilePath] = useState("");
  const [containerEnvVars, setContainerEnvVars] = useState<{ key: string; value: string }[]>([]);
  const toEnvList = (vars: { key: string; value: string }[]) =>
    vars
      .filter((e) => e.key.trim() !== "")
      .map((e) => `${e.key.trim()}=${e.value.trim()}`);

  const github = useQuery({
    queryKey: ["github-connection"],
    queryFn: fetchGitHubConnection,
  });
  const hasRepos = (github.data?.repos.length ?? 0) > 0;

  const deployContainer = useMutation({
    mutationFn: () =>
      apiFetch<void>(`/agents/${agentId}/containers/${encodeURIComponent(containerName.trim())}`, {
        method: "PUT",
        body: JSON.stringify(
          deploySource === "github"
            ? { source: { repo_url: repoUrlFor(selectedRepo), git_ref: gitRef, dockerfile_path: dockerfilePath }, env: toEnvList(containerEnvVars) }
            : { image, env: toEnvList(containerEnvVars) },
        ),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["containers", agentId] });
      setShowContainerForm(false);
      setContainerName("");
      setImage("");
      setSelectedRepo("");
      setGitRef("");
      setDockerfilePath("");
      setContainerEnvVars([]);
      setNotice({ text: "Container queued for deployment.", tone: "info" });
    },
  });
  const removeContainer = useMutation({
    mutationFn: (name: string) =>
      apiFetch<void>(`/agents/${agentId}/containers/${encodeURIComponent(name)}`, { method: "DELETE" }),
    onSuccess: (_data, name) => {
      queryClient.invalidateQueries({ queryKey: ["containers", agentId] });
      setNotice({ text: `Container "${name}" queued for removal.`, tone: "info" });
      setConfirmAction(null);
    },
    onError: (error, name) => {
      setNotice({ text: `Couldn't remove container "${name}": ${(error as Error).message}`, tone: "error" });
      setConfirmAction(null);
    },
  });

  const composeStacks = useQuery({
    queryKey: ["compose-stacks", agentId],
    queryFn: () => apiFetch<ComposeStacksResponse>(`/agents/${agentId}/compose-stacks`),
    refetchInterval: 5000,
  });
  const activeCompose = (composeStacks.data?.desired ?? []).filter((d) => {
    if (d.status !== "absent") return true;
    const observed = composeStacks.data?.observed.find((o) => o.name === d.name);
    return observed !== undefined && observed.status !== "removed";
  });

  const [composeName, setComposeName] = useState("");
  const [composeSelectedRepo, setComposeSelectedRepo] = useState("");
  const [composeGitRef, setComposeGitRef] = useState("");
  const [composeFilePath, setComposeFilePath] = useState("");
  const [composeEnvVars, setComposeEnvVars] = useState<{ key: string; value: string }[]>([]);

  const deployComposeStack = useMutation({
    mutationFn: () =>
      apiFetch<void>(`/agents/${agentId}/compose-stacks/${encodeURIComponent(composeName.trim())}`, {
        method: "PUT",
        body: JSON.stringify({
          repo_url: repoUrlFor(composeSelectedRepo),
          git_ref: composeGitRef,
          compose_file_path: composeFilePath,
          env: toEnvList(composeEnvVars),
        }),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["compose-stacks", agentId] });
      setShowComposeForm(false);
      setComposeName("");
      setComposeSelectedRepo("");
      setComposeGitRef("");
      setComposeFilePath("");
      setComposeEnvVars([]);
      setNotice({ text: "Compose stack queued for deployment.", tone: "info" });
    },
  });

  const removeComposeStack = useMutation({
    mutationFn: (name: string) =>
      apiFetch<void>(`/agents/${agentId}/compose-stacks/${encodeURIComponent(name)}`, { method: "DELETE" }),
    onSuccess: (_data, name) => {
      queryClient.invalidateQueries({ queryKey: ["compose-stacks", agentId] });
      setNotice({ text: `Stack "${name}" queued for removal.`, tone: "info" });
      setConfirmAction(null);
    },
    onError: (error, name) => {
      setNotice({ text: `Couldn't remove stack "${name}": ${(error as Error).message}`, tone: "error" });
      setConfirmAction(null);
    },
  });

  const [routeName, setRouteName] = useState("");
  const [serverName, setServerName] = useState("");
  const [listenPort, setListenPort] = useState(80);
  const [upstreamHost, setUpstreamHost] = useState("127.0.0.1");
  const [upstreamPort, setUpstreamPort] = useState(8080);
  const deployRoute = useMutation({    mutationFn: () =>
      apiFetch<void>(`/agents/${agentId}/proxy-routes/${encodeURIComponent(routeName.trim())}`, {
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
      setShowRouteForm(false);
      setRouteName("");
      setNotice({ text: "Proxy route queued.", tone: "info" });
    },
  });
  const removeRoute = useMutation({
    mutationFn: (name: string) =>
      apiFetch<void>(`/agents/${agentId}/proxy-routes/${encodeURIComponent(name)}`, { method: "DELETE" }),
    onSuccess: (_data, name) => {
      queryClient.invalidateQueries({ queryKey: ["proxy-routes", agentId] });
      setNotice({ text: `Route "${name}" queued for removal.`, tone: "info" });
      setConfirmAction(null);
    },
    onError: (error, name) => {
      setNotice({ text: `Couldn't remove route "${name}": ${(error as Error).message}`, tone: "error" });
      setConfirmAction(null);
    },
  });

  const images = useQuery({
    queryKey: ["images", agentId],
    queryFn: () => apiFetch<ImagesDto>(`/agents/${agentId}/images`),
    staleTime: 0,
    refetchInterval: 20000,
  });
  const deleteImage = useMutation({
    mutationFn: (id: string) =>
      apiFetch<ImagesDto>(`/agents/${agentId}/images/${encodeURIComponent(id)}`, { method: "DELETE" }),
    onSuccess: (data) => {
      queryClient.setQueryData(["images", agentId], data);
      if (data.error) {
        setNotice({ text: data.error, tone: "error" });
      } else {
        setNotice({ text: "Image deleted.", tone: "info" });
      }
      setConfirmAction(null);
    },
    onError: (error) => {
      setNotice({ text: `Couldn't delete image: ${(error as Error).message}`, tone: "error" });
      setConfirmAction(null);
    },
  });

  function handleDeployContainer(e: FormEvent) {
    e.preventDefault();
    deployContainer.mutate();
  }
  function handleDeployComposeStack(e: FormEvent) {
    e.preventDefault();
    deployComposeStack.mutate();
  }
  function handleDeployRoute(e: FormEvent) {
    e.preventDefault();
    deployRoute.mutate();
  }

  function handleConfirmAction() {
    if (!confirmAction) return;
    switch (confirmAction.kind) {
      case "container":
        removeContainer.mutate(confirmAction.name);
        break;
      case "compose":
        removeComposeStack.mutate(confirmAction.name);
        break;
      case "route":
        removeRoute.mutate(confirmAction.name);
        break;
      case "image":
        deleteImage.mutate(confirmAction.id);
        break;
      case "revoke":
        revoke.mutate();
        break;
    }
  }

  const sprite = agentId ? spriteFor(agentId) : null;

  const deploymentCount = activeContainers.length + activeCompose.length;
  const statChips: string[] = [];
  if (containers.data && composeStacks.data) {
    statChips.push(`${deploymentCount} DEPLOYMENT${deploymentCount === 1 ? "" : "S"}`);
  }
  if (proxyRoutes.data) {
    const n = proxyRoutes.data.desired.length;
    statChips.push(`${n} ROUTE${n === 1 ? "" : "S"}`);
  }

  const confirmBusy =
    removeContainer.isPending ||
    removeComposeStack.isPending ||
    removeRoute.isPending ||
    deleteImage.isPending ||
    revoke.isPending;

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

      <main style={{ maxWidth: 1180, margin: "0 auto", padding: "36px 32px 90px" }}>
        {notice && (
          <div className={notice.tone === "error" ? "alert-row" : "alert-row alert-row-info"} style={{ marginBottom: 20 }} role="status">
            {notice.text}
          </div>
        )}

        <section style={{ marginBottom: 26 }}>
          <div className="pixel-panel" style={{ padding: 0, overflow: "hidden" }}>
            <div
              style={{
                background: "linear-gradient(180deg, #DCEBFF 0%, var(--bg) 100%)",
                padding: "26px 30px 20px",
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
              <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 10 }}>
                {statChips.length > 0 && (
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>
                    {statChips.map((chip) => (
                      <span key={chip} className="badge">{chip}</span>
                    ))}
                  </div>
                )}
                {agent && (
                  <button
                    type="button"
                    className="pixel-btn pixel-btn-danger"
                    onClick={() => setConfirmAction({ kind: "revoke" })}
                  >
                    REVOKE AGENT
                  </button>
                )}
              </div>
            </div>

            <div style={{ padding: "18px 30px" }}>
              <p className="mono" style={{ fontSize: 11.5, color: "var(--muted)", margin: 0 }}>
                ⓘ Changes are queued and applied when this agent next checks in — usually within one heartbeat.
              </p>
            </div>
          </div>
        </section>

        {isRevoked && (
          <div className="alert-row alert-row-warn" style={{ marginBottom: 22 }}>
            This agent is revoked — deploying new containers, stacks or routes is disabled.
          </div>
        )}

        <SectionPanel
          title="DEPLOYMENTS"
          count={containers.data || composeStacks.data ? deploymentCount : undefined}
          action={
            !isRevoked && (
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                <button
                  type="button"
                  className={deployTab === "container" ? "pixel-btn pixel-btn-sm" : "pixel-btn pixel-btn-ghost pixel-btn-sm"}
                  onClick={() => setDeployTab("container")}
                >
                  CONTAINERS
                </button>
                <button
                  type="button"
                  className={deployTab === "compose" ? "pixel-btn pixel-btn-sm" : "pixel-btn pixel-btn-ghost pixel-btn-sm"}
                  onClick={() => setDeployTab("compose")}
                >
                  COMPOSE
                </button>
                {deployTab === "container" ? (
                  <button
                    type="button"
                    className={showContainerForm ? "pixel-btn pixel-btn-ghost pixel-btn-sm" : "pixel-btn pixel-btn-sm"}
                    onClick={() => setShowContainerForm((v) => !v)}
                  >
                    {showContainerForm ? "✕ CLOSE" : "+ NEW CONTAINER"}
                  </button>
                ) : (
                  <button
                    type="button"
                    className={showComposeForm ? "pixel-btn pixel-btn-ghost pixel-btn-sm" : "pixel-btn pixel-btn-sm"}
                    onClick={() => setShowComposeForm((v) => !v)}
                  >
                    {showComposeForm ? "✕ CLOSE" : "+ NEW STACK"}
                  </button>
                )}
              </div>
            )
          }
        >
          {deployTab === "container" ? (
            <>
              {containers.isLoading && <EmptyHint>Loading containers…</EmptyHint>}
              {containers.isError && (
                <div className="alert-row">Couldn't load containers — retrying… ({(containers.error as Error).message})</div>
              )}
              {containers.data && activeContainers.length === 0 && (
                <EmptyHint>No containers yet{isRevoked ? "." : " — use “+ New Container” to deploy one."}</EmptyHint>
              )}
              {activeContainers.length > 0 && (
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
                          <td style={{ fontWeight: 700 }}>{d.name}</td>
                          <td>
                            {d.source ? (
                              <RepoTag repoUrl={d.source.repo_url} gitRef={d.source.git_ref} />
                            ) : (
                              d.image
                            )}
                            {(d.env?.length ?? 0) > 0 && (
                              <div className="mono" style={{ fontSize: 10.5, color: "var(--muted)", marginTop: 3 }}>
                                {d.env!.length} env var{d.env!.length === 1 ? "" : "s"}
                              </div>
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
                            <div style={{ display: "flex", gap: 6, flexWrap: "wrap", justifyContent: "flex-end" }}>
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
                                onClick={() => setConfirmAction({ kind: "container", name: d.name })}
                                disabled={isAbsent}
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
              )}

              <CollapsibleForm
                title="NEW CONTAINER"
                open={showContainerForm && !isRevoked}
                onCancel={() => setShowContainerForm(false)}
              >
                <form onSubmit={handleDeployContainer}>
                  <div style={{ display: "flex", gap: 10, flexWrap: "wrap", alignItems: "flex-end", marginBottom: 14 }}>
                    <Field label="NAME">
                      <input placeholder="my-app" value={containerName} onChange={(e) => setContainerName(e.target.value)} required autoFocus />
                    </Field>
                    <div style={{ display: "flex", gap: 8 }}>
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
                        BUILD FROM GITHUB
                      </button>
                    </div>
                  </div>
                  {deploySource === "image" ? (
                    <div style={{ display: "flex", gap: 10, flexWrap: "wrap", marginBottom: 14 }}>
                      <Field label="IMAGE">
                        <input placeholder="nginx:alpine" value={image} onChange={(e) => setImage(e.target.value)} required />
                      </Field>
                    </div>
                  ) : hasRepos ? (
                    <div style={{ display: "flex", gap: 10, flexWrap: "wrap", marginBottom: 14 }}>
                      <Field label="REPOSITORY">
                        <select value={selectedRepo} onChange={(e) => setSelectedRepo(e.target.value)} required>
                          <option value="" disabled>
                            select a repo…
                          </option>
                          {github.data?.repos.map((r) => (
                            <option key={r.full_name} value={r.full_name}>
                              {r.full_name}
                              {r.private ? " (private)" : ""}
                            </option>
                          ))}
                        </select>
                      </Field>
                      <Field label="BRANCH" optional>
                        <input placeholder="repo's default branch" value={gitRef} onChange={(e) => setGitRef(e.target.value)} />
                      </Field>
                      <Field label="DOCKERFILE PATH" optional>
                        <input placeholder="Dockerfile" value={dockerfilePath} onChange={(e) => setDockerfilePath(e.target.value)} />
                      </Field>
                    </div>
                  ) : (
                    <p className="mono" style={{ fontSize: 11.5, color: "var(--muted)", margin: "0 0 14px" }}>
                      {github.isLoading ? "Loading repos…" : "Connect a GitHub account from the dashboard first."}
                    </p>
                  )}
                  <div style={{ marginBottom: 14 }}>
                    <EnvVarsEditor vars={containerEnvVars} onChange={setContainerEnvVars} />
                  </div>
                  <button
                    type="submit"
                    className="pixel-btn pixel-btn-sm"
                    disabled={deployContainer.isPending || (deploySource === "github" && !hasRepos)}
                  >
                    {deployContainer.isPending ? "DEPLOYING…" : "DEPLOY"}
                  </button>
                  {deployContainer.isError && <div className="alert-row">{(deployContainer.error as Error).message}</div>}
                </form>
              </CollapsibleForm>
            </>
          ) : (
            <>
              {composeStacks.isLoading && <EmptyHint>Loading stacks…</EmptyHint>}
              {composeStacks.isError && (
                <div className="alert-row">Couldn't load stacks — retrying… ({(composeStacks.error as Error).message})</div>
              )}
              {composeStacks.data && activeCompose.length === 0 && (
                <EmptyHint>No compose stacks yet{isRevoked ? "." : " — use “+ New Stack” to deploy a multi-container app."}</EmptyHint>
              )}
              {activeCompose.length > 0 && (
                <table>
                  <thead>
                    <tr>
                      <th>NAME</th>
                      <th>SOURCE</th>
                      <th>DESIRED</th>
                      <th>OBSERVED</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {activeCompose.map((d) => {
                      const observed = composeStacks.data?.observed.find((o) => o.name === d.name);
                      const isAbsent = d.status === "absent";
                      return (
                        <tr key={d.name}>
                          <td style={{ fontWeight: 700 }}>{d.name}</td>
                          <td>
                            {d.source ? (
                              <>
                                <RepoTag repoUrl={d.source.repo_url} gitRef={d.source.git_ref} />
                                {d.compose_file_path && (
                                  <div className="mono" style={{ fontSize: 10.5, color: "var(--muted)", marginTop: 3 }}>
                                    file: {d.compose_file_path}
                                  </div>
                                )}
                              </>
                            ) : (
                              "—"
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
                            <div style={{ display: "flex", gap: 6, flexWrap: "wrap", justifyContent: "flex-end" }}>
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
                                onClick={() => setConfirmAction({ kind: "compose", name: d.name })}
                                disabled={isAbsent}
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
              )}

              <CollapsibleForm
                title="NEW COMPOSE STACK"
                open={showComposeForm && !isRevoked}
                onCancel={() => setShowComposeForm(false)}
              >
                <form onSubmit={handleDeployComposeStack}>
                  <div style={{ display: "flex", gap: 10, flexWrap: "wrap", marginBottom: 14 }}>
                    <Field label="STACK NAME">
                      <input placeholder="lowercase, digits, - and _" value={composeName} onChange={(e) => setComposeName(e.target.value)} required autoFocus />
                    </Field>
                    <Field label="REPOSITORY">
                      <select value={composeSelectedRepo} onChange={(e) => setComposeSelectedRepo(e.target.value)} required disabled={!hasRepos}>
                        <option value="" disabled>
                          select a repo…
                        </option>
                        {github.data?.repos.map((r) => (
                          <option key={r.full_name} value={r.full_name}>
                            {r.full_name}
                            {r.private ? " (private)" : ""}
                          </option>
                        ))}
                      </select>
                    </Field>
                    <Field label="BRANCH" optional>
                      <input placeholder="repo's default branch" value={composeGitRef} onChange={(e) => setComposeGitRef(e.target.value)} />
                    </Field>
                    <Field label="COMPOSE FILE" optional>
                      <input placeholder="docker-compose.yml" value={composeFilePath} onChange={(e) => setComposeFilePath(e.target.value)} />
                    </Field>
                  </div>
                  {!hasRepos && (
                    <p className="mono" style={{ fontSize: 11.5, color: "var(--muted)", margin: "0 0 14px" }}>
                      {github.isLoading ? "Loading repos…" : "Connect a GitHub account from the dashboard first."}
                    </p>
                  )}
                  <div style={{ marginBottom: 14 }}>
                    <EnvVarsEditor vars={composeEnvVars} onChange={setComposeEnvVars} />
                  </div>
                  <button type="submit" className="pixel-btn pixel-btn-sm" disabled={deployComposeStack.isPending || !hasRepos}>
                    {deployComposeStack.isPending ? "DEPLOYING…" : "DEPLOY STACK"}
                  </button>
                  {deployComposeStack.isError && <div className="alert-row">{(deployComposeStack.error as Error).message}</div>}
                </form>
              </CollapsibleForm>
            </>
          )}
        </SectionPanel>

        <SectionPanel
          title="PROXY ROUTES"
          count={proxyRoutes.data ? proxyRoutes.data.desired.length : undefined}
          action={
            !isRevoked && (
              <button
                type="button"
                className={showRouteForm ? "pixel-btn pixel-btn-ghost pixel-btn-sm" : "pixel-btn pixel-btn-sm"}
                onClick={() => setShowRouteForm((v) => !v)}
              >
                {showRouteForm ? "✕ CLOSE" : "+ NEW ROUTE"}
              </button>
            )
          }
        >
          {proxyRoutes.data?.error && (
            <div className="alert-row">Last apply error: {proxyRoutes.data.error}</div>
          )}
          {proxyRoutes.isLoading && <EmptyHint>Loading routes…</EmptyHint>}
          {proxyRoutes.isError && (
                <div className="alert-row">Couldn't load routes — retrying… ({(proxyRoutes.error as Error).message})</div>
          )}
          {proxyRoutes.data && proxyRoutes.data.desired.length === 0 && (
            <EmptyHint>No routes yet{isRevoked ? "." : " — use “+ New Route” to expose a container port."}</EmptyHint>
          )}
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
                    <td style={{ fontWeight: 700 }}>{r.name}</td>
                    <td>{r.server_name || "(catch-all)"}</td>
                    <td>:{r.listen_port}</td>
                    <td>
                      {r.upstream_host}:{r.upstream_port}
                    </td>
                    <td>
                      <div style={{ display: "flex", justifyContent: "flex-end" }}>
                        <button
                          type="button"
                          className="pixel-btn pixel-btn-danger pixel-btn-sm"
                          onClick={() => setConfirmAction({ kind: "route", name: r.name })}
                        >
                          REMOVE
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          <CollapsibleForm
            title="NEW PROXY ROUTE"
            open={showRouteForm && !isRevoked}
            onCancel={() => setShowRouteForm(false)}
          >
            <form onSubmit={handleDeployRoute}>
              <div style={{ display: "flex", gap: 10, flexWrap: "wrap", marginBottom: 14 }}>
                <Field label="NAME">
                  <input placeholder="web" value={routeName} onChange={(e) => setRouteName(e.target.value)} required autoFocus />
                </Field>
                <Field label="SERVER NAME" optional>
                  <input placeholder="example.com" value={serverName} onChange={(e) => setServerName(e.target.value)} />
                </Field>
                <Field label="LISTEN PORT">
                  <input
                    type="number"
                    min={1}
                    max={65535}
                    value={listenPort}
                    onChange={(e) => setListenPort(Number(e.target.value))}
                    required
                  />
                </Field>
                <Field label="UPSTREAM HOST">
                  <input placeholder="127.0.0.1" value={upstreamHost} onChange={(e) => setUpstreamHost(e.target.value)} required />
                </Field>
                <Field label="UPSTREAM PORT">
                  <input
                    type="number"
                    min={1}
                    max={65535}
                    value={upstreamPort}
                    onChange={(e) => setUpstreamPort(Number(e.target.value))}
                    required
                  />
                </Field>
              </div>
              <button type="submit" className="pixel-btn pixel-btn-sm" disabled={deployRoute.isPending}>
                {deployRoute.isPending ? "DEPLOYING…" : "DEPLOY ROUTE"}
              </button>
              {deployRoute.isError && <div className="alert-row">{(deployRoute.error as Error).message}</div>}
            </form>
          </CollapsibleForm>
        </SectionPanel>

        <SectionPanel
          title="IMAGES"
          count={images.data ? images.data.images.length : undefined}
          action={
            <button
              type="button"
              className="pixel-btn pixel-btn-ghost pixel-btn-sm"
              onClick={() => images.refetch()}
              disabled={images.isFetching}
            >
              {images.isFetching ? "…" : "↺ REFRESH"}
            </button>
          }
        >
          {images.data?.error && <div className="alert-row">{images.data.error}</div>}
          {images.isLoading && <EmptyHint>Loading images…</EmptyHint>}
          {images.isError && (
            <div className="alert-row">Couldn't load images — retrying… ({(images.error as Error).message})</div>
          )}
          {images.data && !images.data.error && images.data.images.length === 0 && (
            <EmptyHint>No images on this host yet.</EmptyHint>
          )}
          {images.data && images.data.images.length > 0 && (
            <table>
              <thead>
                <tr>
                  <th>IMAGE</th>
                  <th>SIZE</th>
                  <th>CREATED</th>
                  <th>STATUS</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {images.data.images.map((img) => {
                  const label = imageLabel(img);
                  return (
                    <tr key={img.id}>
                      <td>
                        <div style={{ fontWeight: 700, wordBreak: "break-all" }}>{label}</div>
                        {(img.repo_tags.length > 1 || img.repo_tags.length === 0) && (
                          <div className="mono" style={{ fontSize: 10.5, color: "var(--muted)", marginTop: 3 }}>
                            {img.repo_tags.length > 1
                              ? `+${img.repo_tags.length - 1} more tag${img.repo_tags.length === 2 ? "" : "s"}`
                              : "dangling"}
                          </div>
                        )}
                      </td>
                      <td>{formatSize(img.size_bytes)}</td>
                      <td>{new Date(img.created_at * 1000).toLocaleDateString()}</td>
                      <td>
                        <span
                          className="badge"
                          style={
                            img.in_use
                              ? { background: "#E4F9EE", color: "var(--hp-dark)", borderColor: "var(--hp-dark)" }
                              : { background: "#F3F0EA", color: "#8A7E72", borderColor: "#8A7E72" }
                          }
                        >
                          {img.in_use ? "IN USE" : "UNUSED"}
                        </span>
                      </td>
                      <td>
                        <div style={{ display: "flex", justifyContent: "flex-end" }}>
                          <button
                            type="button"
                            className="pixel-btn pixel-btn-danger pixel-btn-sm"
                            onClick={() => setConfirmAction({ kind: "image", id: img.id, label })}
                            disabled={img.in_use}
                            title={img.in_use ? "Stop/remove the containers using this image first" : undefined}
                          >
                            DELETE
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </SectionPanel>
      </main>

      {logsModal && agentId && (
        <LogsModal
          agentId={agentId}
          containerName={logsModal.containerName}
          onClose={() => setLogsModal(null)}
        />
      )}

      {confirmAction && !isRevoked && (
        <ConfirmDialog
          action={confirmAction}
          busy={confirmBusy}
          onConfirm={handleConfirmAction}
          onCancel={() => setConfirmAction(null)}
        />
      )}
    </div>
  );
}
