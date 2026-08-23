-- GitHub OAuth App integration, Phase 1 (connect account + list repos —
-- deploy-from-repo wiring is a later migration). See docs/ (github
-- integration writeup, once added) and the pairing_tokens table this
-- mirrors for the state-token pattern.

-- Single-use, short-TTL CSRF state for the OAuth redirect round trip.
-- Same shape and consumption pattern as pairing_tokens: the plaintext
-- state value is never stored, only its hash, and `POST /github/oauth/start`
-- through `GET /github/oauth/callback` is exactly the "issue, then
-- row-locked consume" flow `register_agent` already uses for pairing
-- tokens — reused here because the callback is a real browser redirect
-- from github.com and can't carry the dashboard's normal Authorization
-- header, so this is the only way the callback learns which account
-- started the flow.
CREATE TABLE github_oauth_states (
    state_hash BYTEA PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ
);

-- One GitHub account linked per Harbory account. access_token is a
-- standard GitHub OAuth App user token (not a short-lived installation
-- token — that's only available to GitHub Apps, not OAuth Apps) with
-- `repo` scope, so it can read both public and private repos the user
-- can access. Treat this column with the same care as any other secret
-- in this database (see docs/security.md) — nothing here is encrypted at
-- rest beyond whatever protects the database itself, consistent with
-- this project's "no extra infra" stance rather than adding a
-- field-level encryption layer for one column.
CREATE TABLE github_connections (
    account_id UUID PRIMARY KEY REFERENCES accounts(id),
    access_token TEXT NOT NULL,
    github_login TEXT NOT NULL,
    connected_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
