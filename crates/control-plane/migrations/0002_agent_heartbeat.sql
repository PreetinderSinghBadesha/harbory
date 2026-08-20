-- Online/offline is computed at read time from this timestamp (now() minus
-- last_heartbeat_at compared against a threshold), rather than maintained
-- as a separate status flag by a background sweeper — simpler and avoids
-- the sweeper drifting out of sync. See /docs/connection-lifecycle.md.
ALTER TABLE agents ADD COLUMN last_heartbeat_at TIMESTAMPTZ;
