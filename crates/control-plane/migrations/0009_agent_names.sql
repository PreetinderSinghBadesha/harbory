-- User-settable display name for an agent. Nullable: newly registered
-- agents always get a generated default (see src/names.rs), but existing
-- rows from before this migration have none — reads fall back to a
-- derived-from-id name at query time (see store/agents.rs) rather than
-- backfilling, so there's nothing to keep in sync.
ALTER TABLE agents ADD COLUMN name TEXT;
