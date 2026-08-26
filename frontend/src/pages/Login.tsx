import { useState, type FormEvent } from "react";
import { Link, Navigate } from "react-router-dom";
import { supabase, supabaseConfigured } from "../lib/supabase";
import { useAuth } from "../context/AuthContext";
import { PasswordInput } from "../components/PasswordInput";
import { LoadingSpinner } from "../components/LoadingSpinner";
import "../styles/GameHud.css";
import "./Login.css";

function GoogleIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 48 48" aria-hidden="true">
      <path fill="#FFC107" d="M43.6 20.5H42V20H24v8h11.3c-1.6 4.7-6.1 8-11.3 8-6.6 0-12-5.4-12-12s5.4-12 12-12c3.1 0 5.8 1.1 8 3l5.7-5.7C34.6 6.1 29.6 4 24 4 12.9 4 4 12.9 4 24s8.9 20 20 20 20-8.9 20-20c0-1.3-.1-2.7-.4-3.5z" />
      <path fill="#FF3D00" d="m6.3 14.7 6.6 4.8C14.6 15.9 18.9 13 24 13c3.1 0 5.8 1.1 8 3l5.7-5.7C34.6 6.1 29.6 4 24 4c-7.4 0-13.8 4.1-17.1 10.1z" />
      <path fill="#4CAF50" d="M24 44c5.5 0 10.4-1.9 14.2-5.1l-6.6-5.6c-2.1 1.5-4.7 2.4-7.6 2.4-5.2 0-9.6-3.3-11.3-7.9l-6.6 5.1C9.9 39.6 16.4 44 24 44z" />
      <path fill="#1976D2" d="M43.6 20.5H42V20H24v8h11.3c-.8 2.2-2.2 4.1-4.1 5.4l6.6 5.6C41.6 35.7 44 30.3 44 24c0-1.3-.1-2.7-.4-3.5z" />
    </svg>
  );
}

export function Login() {
  const { session } = useAuth();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [mode, setMode] = useState<"signin" | "signup">("signin");
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [googleLoading, setGoogleLoading] = useState(false);

  if (session) return <Navigate to="/dashboard" replace />;

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setInfo(null);
    setSubmitting(true);
    try {
      const { error } =
        mode === "signin"
          ? await supabase.auth.signInWithPassword({ email, password })
          : await supabase.auth.signUp({ email, password });
      if (error) {
        setError(error.message);
      } else if (mode === "signup") {
        setInfo("Check your email to confirm your account, then sign in.");
      }
    } finally {
      setSubmitting(false);
    }
  }

  async function handleGoogle() {
    setError(null);
    setInfo(null);
    setGoogleLoading(true);
    // Real navigation on success — Supabase redirects the browser to
    // Google and back, so there's nothing to clean up locally; only a
    // failure to even start the redirect leaves us on this page.
    const { error } = await supabase.auth.signInWithOAuth({
      provider: "google",
      options: { redirectTo: `${window.location.origin}/dashboard` },
    });
    if (error) {
      setError(error.message);
      setGoogleLoading(false);
    }
  }

  return (
    <div className="game-hud login-page">
      <div className="pixel-panel login-card">
        <Link
          to="/"
          className="pixel"
          style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 9, fontSize: 13, marginBottom: 28 }}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--clay)" strokeWidth="1.8" strokeLinejoin="round" strokeLinecap="round" aria-hidden="true">
            <path d="M12 2.5 L21 7.5 V16.5 L12 21.5 L3 16.5 V7.5 Z" />
            <path d="M3 7.5 L12 12.5 L21 7.5" />
            <path d="M12 12.5 V21.5" />
          </svg>
          HARBORY
        </Link>

        <div style={{ textAlign: "center", marginBottom: 26 }}>
          <h1 style={{ fontSize: 22, fontWeight: 800, letterSpacing: "-0.01em", margin: "0 0 6px" }}>
            {mode === "signin" ? "Welcome back" : "Create your account"}
          </h1>
          <p style={{ fontSize: 13.5, color: "var(--muted)", margin: 0 }}>
            {mode === "signin" ? "Sign in to manage your agents." : "Get started pairing your first agent."}
          </p>
        </div>

        {!supabaseConfigured && (
          <div className="alert-row alert-row-warn" style={{ marginBottom: 20 }}>
            Supabase isn&apos;t configured yet. Copy <code>frontend/.env.example</code> to{" "}
            <code>frontend/.env</code> and fill in your project&apos;s URL and anon key — see docs/dashboard.md.
          </div>
        )}

        <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          <label style={{ display: "flex", flexDirection: "column", gap: 6, fontSize: 12.5, color: "var(--muted)", fontWeight: 600 }}>
            <span>Email</span>
            <input
              type="email"
              placeholder="you@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
            />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 6, fontSize: 12.5, color: "var(--muted)", fontWeight: 600 }}>
            <span>Password</span>
            <PasswordInput
              placeholder="••••••••"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={6}
            />
          </label>
          <button type="submit" className="pixel-btn" style={{ width: "100%", justifyContent: "center" }} disabled={submitting}>
            {submitting && <LoadingSpinner dotColor="var(--panel)" />}
            {submitting ? "PLEASE WAIT…" : mode === "signin" ? "SIGN IN" : "SIGN UP"}
          </button>
        </form>

        <div className="login-divider">
          <span>or</span>
        </div>

        <button
          type="button"
          className="pixel-btn pixel-btn-ghost"
          style={{ width: "100%", justifyContent: "center", gap: 10 }}
          onClick={handleGoogle}
          disabled={googleLoading}
        >
          {googleLoading ? <LoadingSpinner /> : <GoogleIcon />}
          {googleLoading ? "REDIRECTING…" : "CONTINUE WITH GOOGLE"}
        </button>

        {error && <div className="alert-row">{error}</div>}
        {info && <div className="alert-row alert-row-info">{info}</div>}

        <button
          type="button"
          onClick={() => setMode(mode === "signin" ? "signup" : "signin")}
          style={{
            display: "block",
            width: "100%",
            marginTop: 20,
            background: "none",
            border: "none",
            color: "var(--muted)",
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            fontWeight: 600,
            textAlign: "center",
            cursor: "pointer",
            padding: 0,
          }}
        >
          {mode === "signin" ? "Need an account? Sign up" : "Already have an account? Sign in"}
        </button>
      </div>
    </div>
  );
}
