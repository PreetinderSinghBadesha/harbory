import { useState, type FormEvent } from "react";
import { Link, Navigate } from "react-router-dom";
import { supabase, supabaseConfigured } from "../lib/supabase";
import { useAuth } from "../context/AuthContext";
import "../styles/GameHud.css";
import "./Login.css";

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
            <input
              type="password"
              placeholder="••••••••"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={6}
            />
          </label>
          <button type="submit" className="pixel-btn" style={{ width: "100%", justifyContent: "center" }} disabled={submitting}>
            {submitting ? "PLEASE WAIT…" : mode === "signin" ? "SIGN IN" : "SIGN UP"}
          </button>
        </form>

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
