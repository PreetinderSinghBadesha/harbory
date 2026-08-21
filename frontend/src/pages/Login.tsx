import { useState, type FormEvent } from "react";
import { Navigate } from "react-router-dom";
import { supabase, supabaseConfigured } from "../lib/supabase";
import { useAuth } from "../context/AuthContext";

export function Login() {
  const { session } = useAuth();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [mode, setMode] = useState<"signin" | "signup">("signin");
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  if (session) return <Navigate to="/" replace />;

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
      <h1>Harbory</h1>
      {!supabaseConfigured && (
        <p className="error">
          Supabase isn't configured yet. Copy <code>frontend/.env.example</code> to{" "}
          <code>frontend/.env</code> and fill in your project's URL and anon key — see
          docs/dashboard.md.
        </p>
      )}
      <form onSubmit={handleSubmit}>
        <input
          type="email"
          placeholder="Email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          required
        />
        <input
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
          minLength={6}
        />
        <button type="submit" disabled={submitting}>
          {mode === "signin" ? "Sign in" : "Sign up"}
        </button>
      </form>
      <button type="button" onClick={handleGitHub}>
        Continue with GitHub
      </button>
      <button type="button" className="link-button" onClick={() => setMode(mode === "signin" ? "signup" : "signin")}>
        {mode === "signin" ? "Need an account? Sign up" : "Already have an account? Sign in"}
      </button>
      {error && <p className="error">{error}</p>}
      {info && <p className="info">{info}</p>}
    </div>
  );
}
