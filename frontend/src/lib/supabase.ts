import { createClient } from "@supabase/supabase-js";

const url = import.meta.env.VITE_SUPABASE_URL as string | undefined;
const anonKey = import.meta.env.VITE_SUPABASE_ANON_KEY as string | undefined;

if (!url || !anonKey) {
  // Doesn't throw: lets the app still render (e.g. to show a clear setup
  // error on the login page) rather than a blank white screen.
  console.warn(
    "VITE_SUPABASE_URL / VITE_SUPABASE_ANON_KEY are not set. Copy frontend/.env.example to " +
      "frontend/.env and fill in your Supabase project's values — see docs/dashboard.md.",
  );
}

export const supabaseConfigured = Boolean(url && anonKey);

export const supabase = createClient(url ?? "https://placeholder.invalid", anonKey ?? "placeholder");
