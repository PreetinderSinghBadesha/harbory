import { useState, type FormEvent } from "react";
import { Link, Navigate } from "react-router-dom";
import { supabase, supabaseConfigured } from "../lib/supabase";
import { useAuth } from "../context/AuthContext";
import "./Login.css";

function GitHubIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 .5C5.73.5.98 5.24.98 11.52c0 5.02 3.26 9.28 7.77 10.78.57.1.78-.25.78-.55 0-.27-.01-1.16-.02-2.11-3.16.69-3.83-1.34-3.83-1.34-.52-1.31-1.26-1.66-1.26-1.66-1.03-.7.08-.69.08-.69 1.14.08 1.74 1.17 1.74 1.17 1.01 1.74 2.66 1.24 3.31.95.1-.73.4-1.24.72-1.53-2.52-.29-5.17-1.26-5.17-5.61 0-1.24.44-2.25 1.17-3.05-.12-.29-.51-1.45.11-3.02 0 0 .96-.31 3.13 1.16a10.8 10.8 0 0 1 5.7 0c2.17-1.47 3.13-1.16 3.13-1.16.62 1.57.23 2.73.11 3.02.73.8 1.17 1.81 1.17 3.05 0 4.36-2.66 5.32-5.19 5.6.41.36.77 1.06.77 2.14 0 1.54-.01 2.79-.01 3.17 0 .3.2.66.79.55A10.53 10.53 0 0 0 23.02 11.5C23.02 5.24 18.27.5 12 .5Z" />
    </svg>
  );
}

function BrandMark() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--a-accent)" strokeWidth="1.6" strokeLinejoin="round" aria-hidden="true">
      <path d="M12 2.5 L21 7.5 V16.5 L12 21.5 L3 16.5 V7.5 Z" />
      <path d="M3 7.5 L12 12.5 L21 7.5" />
      <path d="M12 12.5 V21.5" />
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

  async function handleGitHub() {
    await supabase.auth.signInWithOAuth({ provider: "github" });
  }

  return (
    <div className="auth-page">
      <div className="auth-bg" aria-hidden="true" />
      <div className="auth-glow" aria-hidden="true" />
      <div className="auth-card">
        <Link to="/" className="auth-brand">
          <BrandMark />
          Harbory
        </Link>

        <div className="auth-head">
          <h1>{mode === "signin" ? "Welcome back" : "Create your account"}</h1>
          <p className="auth-sub">
            {mode === "signin" ? "Sign in to manage your agents." : "Get started pairing your first agent."}
          </p>
        </div>

        {!supabaseConfigured && (
          <div className="auth-alert auth-alert-warn">
            Supabase isn&apos;t configured yet. Copy <code>frontend/.env.example</code> to{" "}
            <code>frontend/.env</code> and fill in your project&apos;s URL and anon key — see docs/dashboard.md.
          </div>
        )}

        <form className="auth-form" onSubmit={handleSubmit}>
          <label className="auth-field">
            <span>Email</span>
            <input
              type="email"
              placeholder="you@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
            />
          </label>
          <label className="auth-field">
            <span>Password</span>
            <input
              type="password"
              placeholder="••••••••"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={6}
            />
          </label>
          <button type="submit" className="auth-btn auth-btn-primary" disabled={submitting}>
            {submitting ? "Please wait…" : mode === "signin" ? "Sign in" : "Sign up"}
          </button>
        </form>

        <div className="auth-divider">or</div>

        <button type="button" className="auth-btn auth-btn-ghost" onClick={handleGitHub}>
          <GitHubIcon />
          Continue with GitHub
        </button>

        {error && <div className="auth-alert auth-alert-error">{error}</div>}
        {info && <div className="auth-alert auth-alert-info">{info}</div>}

        <button
          type="button"
          className="auth-switch"
          onClick={() => setMode(mode === "signin" ? "signup" : "signin")}
        >
          {mode === "signin" ? "Need an account? Sign up" : "Already have an account? Sign in"}
        </button>
      </div>
    </div>
  );
}
