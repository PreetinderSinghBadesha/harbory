-- Desired proxy routes. Unlike desired_containers there's no separate
-- running/absent status: presence of a row *is* "this route should
-- exist," because the whole Nginx config is regenerated from the full
-- route set on every apply, not patched incrementally. Deleting a route
-- means DELETE the row, not soft-marking it absent. See
-- /docs/proxy-management.md.
CREATE TABLE desired_proxy_routes (
    agent_id UUID NOT NULL REFERENCES agents(id),
    name TEXT NOT NULL,
    server_name TEXT NOT NULL DEFAULT '',
    listen_port INTEGER NOT NULL,
    path_prefix TEXT NOT NULL DEFAULT '/',
    upstream_host TEXT NOT NULL,
    upstream_port INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, name)
);

-- Last-known state as reported by the agent, mirroring last_heartbeat_at's
-- pattern of "one row per agent, overwritten in place" rather than a log.
CREATE TABLE proxy_state (
    agent_id UUID PRIMARY KEY REFERENCES agents(id),
    applied_hash BYTEA NOT NULL DEFAULT '',
    error TEXT,
    reported_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
