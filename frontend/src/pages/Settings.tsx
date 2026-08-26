import { useEffect, useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { apiFetch } from "../lib/api";
import { supabase } from "../lib/supabase";
import { useAuth } from "../context/AuthContext";
import { fetchGitHubConnection } from "../lib/github";
import { PasswordInput } from "../components/PasswordInput";
import { LoadingSpinner } from "../components/LoadingSpinner";
import "../styles/GameHud.css";

function ProfileSection() {
  const { session } = useAuth();
  return (
    <section style={{ marginBottom: 48 }}>
      <div className="pixel-panel" style={{ padding: "28px 30px" }}>
        <div className="eyebrow">PROFILE</div>
        <table>
          <tbody>
            <tr>
              <td className="mono" style={{ color: "var(--muted)" }}>EMAIL</td>
              <td className="mono">{session?.user.email ?? "—"}</td>
            </tr>
            <tr>
              <td className="mono" style={{ color: "var(--muted)" }}>ACCOUNT ID</td>
              <td className="mono" style={{ wordBreak: "break-all" }}>{session?.user.id ?? "—"}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  );
}

function ChangePasswordSection() {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [mismatch, setMismatch] = useState(false);

  const changePassword = useMutation({
    mutationFn: async () => {
      const { error } = await supabase.auth.updateUser({ password });
      if (error) throw new Error(error.message);
    },
    onSuccess: () => {
      setPassword("");
      setConfirm("");
    },
  });

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (password !== confirm) {
      setMismatch(true);
      return;
    }
    setMismatch(false);
    changePassword.mutate();
  }

  return (
    <section style={{ marginBottom: 48 }}>
      <div className="pixel-panel" style={{ padding: "28px 30px" }}>
        <div className="eyebrow">CHANGE PASSWORD</div>
        <form onSubmit={handleSubmit} className="inline-form">
          <PasswordInput
            placeholder="new password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={6}
            containerStyle={{ flex: "1 1 8rem" }}
          />
          <PasswordInput
            placeholder="confirm new password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            required
            minLength={6}
            containerStyle={{ flex: "1 1 8rem" }}
          />
          <button type="submit" className="pixel-btn pixel-btn-sm" disabled={changePassword.isPending}>
            {changePassword.isPending && <LoadingSpinner dotColor="var(--panel)" />}
            {changePassword.isPending ? "UPDATING…" : "UPDATE"}
          </button>
        </form>
        {mismatch && <div className="alert-row">Passwords don't match.</div>}
        {changePassword.isError && <div className="alert-row">{(changePassword.error as Error).message}</div>}
        {changePassword.isSuccess && <div className="alert-row alert-row-info">Password updated.</div>}
      </div>
    </section>
  );
}

function GitHubSection() {
  const queryClient = useQueryClient();

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

  return (
    <section style={{ marginBottom: 48 }}>
      <div className="pixel-panel" style={{ padding: "28px 30px" }}>
        <div className="eyebrow">GITHUB</div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 26, flexWrap: "wrap" }}>
          {githubLoading ? (
            <LoadingSpinner label="Loading…" />
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
        {githubCallbackResult === "connected" && <div className="alert-row alert-row-info">GitHub account connected.</div>}
        {githubCallbackResult === "error" && <div className="alert-row">Connecting to GitHub failed — try again.</div>}
        {connectGithub.isError && <div className="alert-row">{(connectGithub.error as Error).message}</div>}
        {disconnectGithub.isError && <div className="alert-row">{(disconnectGithub.error as Error).message}</div>}
      </div>
    </section>
  );
}

export function Settings() {
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
            <span className="pixel" style={{ fontSize: 16 }}>SETTINGS</span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
            <Link to="/dashboard" className="pixel-btn pixel-btn-ghost pixel-btn-sm">
              ← BACK
            </Link>
            <button type="button" className="pixel-btn pixel-btn-ghost pixel-btn-sm" onClick={() => supabase.auth.signOut()}>
              SIGN OUT
            </button>
          </div>
        </div>
      </header>

      <main style={{ maxWidth: 1180, margin: "0 auto", padding: "44px 32px 90px" }}>
        <ProfileSection />
        <ChangePasswordSection />
        <GitHubSection />
      </main>
    </div>
  );
}
