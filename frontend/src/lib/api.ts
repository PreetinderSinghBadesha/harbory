import { supabase } from "./supabase";

const API_URL = (import.meta.env.VITE_API_URL as string | undefined) ?? "http://localhost:8080";

/** Every call attaches the current Supabase session's access token as a
 * Bearer credential — the control plane verifies it against the same
 * project's JWT secret (crates/control-plane/src/auth.rs). Throws with
 * the response body's text on any non-2xx status. */
export async function apiFetch<T>(path: string, options: RequestInit = {}): Promise<T> {
  const { data } = await supabase.auth.getSession();
  const token = data.session?.access_token;

  const response = await fetch(`${API_URL}${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new Error(`${options.method ?? "GET"} ${path} failed: ${response.status} ${body}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }
  return (await response.json()) as T;
}
