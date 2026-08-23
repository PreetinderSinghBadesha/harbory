-- Lets a desired container be built from a git repo instead of a plain
-- image pull. Nullable columns on the existing table (not a joined
-- table) — a desired container is either plain-image or git-sourced,
-- never both, mirroring ContainerSpec.git_source being an optional field
-- on the same protobuf message rather than a separate one. `image`
-- itself still holds a value for git-sourced rows too — the synthetic
-- "git+<repo>#<ref>" reconciliation identity computed in http.rs's
-- put_container, not a real pullable reference. See docs/reconciliation.md.
ALTER TABLE desired_containers
    ADD COLUMN git_repo_url TEXT,
    ADD COLUMN git_ref TEXT,
    ADD COLUMN git_dockerfile_path TEXT;
