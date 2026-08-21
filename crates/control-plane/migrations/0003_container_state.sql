-- Desired/observed state model (the reconciliation foundation — see
-- docs/reconciliation.md). Deliberately narrow: a desired container is
-- either 'running' (with a given image/env/ports/command) or 'absent'.
-- No restart policies, scaling, or scheduling — out of scope for v1.

CREATE TABLE desired_containers (
    agent_id UUID NOT NULL REFERENCES agents(id),
    name TEXT NOT NULL,
    image TEXT NOT NULL,
    env TEXT[] NOT NULL DEFAULT '{}',
    -- [{"host_port": 8080, "container_port": 80}, ...]
    port_mappings JSONB NOT NULL DEFAULT '[]',
    command TEXT[] NOT NULL DEFAULT '{}',
    desired_status TEXT NOT NULL CHECK (desired_status IN ('running', 'absent')),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, name)
);

-- Latest known state as reported by the agent. Full snapshot per agent,
-- replaced wholesale on each ContainerStateReport (see
-- Store::replace_observed_containers) rather than merged incrementally —
-- matches the wire format being a full snapshot, not a delta.
CREATE TABLE observed_containers (
    agent_id UUID NOT NULL REFERENCES agents(id),
    name TEXT NOT NULL,
    image TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK (status IN ('running', 'stopped', 'removed', 'error')),
    error TEXT,
    reported_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, name)
);
