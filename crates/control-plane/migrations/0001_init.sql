-- Phase 1 schema: accounts (minimal stand-in until Phase 5 builds real
-- auth), agents, pairing tokens, and an audit log for security-relevant
-- events. See /docs/database.md for rationale.

CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE agents (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id),
    public_key BYTEA NOT NULL,
    public_key_fingerprint BYTEA NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_agents_account_id ON agents(account_id);

CREATE TABLE pairing_tokens (
    -- SHA-256 of the plaintext token. The plaintext is shown to the user
    -- once and never stored, same rationale as password/API-key hashing.
    token_hash BYTEA PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    consumed_by_agent_id UUID REFERENCES agents(id)
);

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    account_id UUID REFERENCES accounts(id),
    agent_id UUID REFERENCES agents(id),
    detail JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_log_account_id ON audit_log(account_id);
