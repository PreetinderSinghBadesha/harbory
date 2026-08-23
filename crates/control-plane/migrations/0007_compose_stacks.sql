CREATE TABLE desired_compose_stacks (
    agent_id UUID NOT NULL REFERENCES agents(id),
    name TEXT NOT NULL,
    repo_url TEXT NOT NULL,
    git_ref TEXT NOT NULL DEFAULT '',
    compose_file_path TEXT NOT NULL DEFAULT 'docker-compose.yml',
    desired_status TEXT NOT NULL CHECK (desired_status IN ('running', 'absent')),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, name)
);

CREATE TABLE observed_compose_stacks (
    agent_id UUID NOT NULL REFERENCES agents(id),
    name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'stopped', 'removed', 'error')),
    error TEXT,
    reported_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, name)
);
