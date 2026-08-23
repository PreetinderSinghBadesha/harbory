//! The reconciliation foundation the roadmap calls out as not-skippable.
//! Deliberately a pure function over plain data — no DB, no gRPC — so it's
//! trivial to test exhaustively and easy to reason about independent of
//! how desired/observed state get in and out of Postgres. See
//! /docs/reconciliation.md.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesiredStatus {
    Running,
    Absent,
}

/// Plain repo/ref/dockerfile-path, no embedded credential — that's added
/// (only for private repos, only in the wire message) at dispatch time in
/// `stream.rs`, never persisted. See `image` below for how this interacts
/// with reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSource {
    pub repo_url: String,
    pub git_ref: String,
    pub dockerfile_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredContainer {
    pub name: String,
    /// A real pullable reference, or — when `git_source` is set — the
    /// synthetic "git+<repo_url>#<git_ref>" identity string `diff` uses to
    /// decide convergence (computed once, in `http.rs`'s `put_container`,
    /// not here — this module stays DB/protocol-agnostic).
    pub image: String,
    pub env: Vec<String>,
    pub ports: Vec<PortMapping>,
    pub command: Vec<String>,
    pub status: DesiredStatus,
    pub git_source: Option<GitSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedStatus {
    Running,
    Stopped,
    Removed,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedContainer {
    pub name: String,
    pub image: String,
    pub status: ObservedStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Create-or-replace, idempotent: the agent is expected to remove any
    /// existing container by this name before creating the new one, so
    /// this is also the correct action for "wrong image running" and
    /// "crashed and needs recreating", not just "doesn't exist yet".
    Deploy(DesiredContainer),
    Remove(String),
}

/// What needs to happen to `observed` to match `desired`. Order is
/// insertion order of `desired`, then any bare removals — not meaningful
/// beyond determinism for tests.
pub fn diff(desired: &[DesiredContainer], observed: &[ObservedContainer]) -> Vec<Action> {
    let mut actions = Vec::new();

    for d in desired {
        let obs = observed.iter().find(|o| o.name == d.name);
        match d.status {
            DesiredStatus::Running => {
                let converged = matches!(
                    obs,
                    Some(o) if o.status == ObservedStatus::Running && o.image == d.image
                );
                if !converged {
                    actions.push(Action::Deploy(d.clone()));
                }
            }
            DesiredStatus::Absent => {
                if let Some(o) = obs {
                    if o.status != ObservedStatus::Removed {
                        actions.push(Action::Remove(d.name.clone()));
                    }
                }
            }
        }
    }

    // Deliberately not handled: an observed container with no desired row
    // at all. That's "something exists that nobody has an opinion about"
    // (e.g. manual intervention, or a name whose desired row was deleted
    // outright rather than marked absent) — the safe default is to leave
    // it alone rather than guess it should be removed.

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running(name: &str, image: &str) -> DesiredContainer {
        DesiredContainer {
            name: name.into(),
            image: image.into(),
            env: vec![],
            ports: vec![],
            command: vec![],
            status: DesiredStatus::Running,
            git_source: None,
        }
    }

    fn absent(name: &str) -> DesiredContainer {
        DesiredContainer { status: DesiredStatus::Absent, ..running(name, "") }
    }

    fn observed(name: &str, image: &str, status: ObservedStatus) -> ObservedContainer {
        ObservedContainer { name: name.into(), image: image.into(), status }
    }

    #[test]
    fn missing_container_is_deployed() {
        let desired = vec![running("web", "nginx:alpine")];
        let actions = diff(&desired, &[]);
        assert_eq!(actions, vec![Action::Deploy(desired[0].clone())]);
    }

    #[test]
    fn already_running_correct_image_is_left_alone() {
        let desired = vec![running("web", "nginx:alpine")];
        let observed = vec![observed("web", "nginx:alpine", ObservedStatus::Running)];
        assert_eq!(diff(&desired, &observed), vec![]);
    }

    #[test]
    fn wrong_image_triggers_redeploy() {
        let desired = vec![running("web", "nginx:latest")];
        let observed = vec![observed("web", "nginx:alpine", ObservedStatus::Running)];
        assert_eq!(diff(&desired, &observed), vec![Action::Deploy(desired[0].clone())]);
    }

    #[test]
    fn stopped_or_errored_container_is_redeployed() {
        let desired = vec![running("web", "nginx:alpine")];
        for status in [ObservedStatus::Stopped, ObservedStatus::Error, ObservedStatus::Removed] {
            let observed = vec![observed("web", "nginx:alpine", status)];
            assert_eq!(
                diff(&desired, &observed),
                vec![Action::Deploy(desired[0].clone())],
                "status {status:?} should trigger a redeploy"
            );
        }
    }

    #[test]
    fn absent_but_still_observed_is_removed() {
        let desired = vec![absent("web")];
        let observed = vec![observed("web", "nginx:alpine", ObservedStatus::Running)];
        assert_eq!(diff(&desired, &observed), vec![Action::Remove("web".into())]);
    }

    #[test]
    fn absent_and_already_removed_needs_no_action() {
        let desired = vec![absent("web")];
        let observed = vec![observed("web", "nginx:alpine", ObservedStatus::Removed)];
        assert_eq!(diff(&desired, &observed), vec![]);
    }

    #[test]
    fn absent_and_never_observed_needs_no_action() {
        let desired = vec![absent("web")];
        assert_eq!(diff(&desired, &[]), vec![]);
    }

    #[test]
    fn observed_container_with_no_desired_row_is_left_alone() {
        let observed = vec![observed("mystery", "whatever", ObservedStatus::Running)];
        assert_eq!(diff(&[], &observed), vec![]);
    }

    #[test]
    fn multiple_containers_are_diffed_independently() {
        let desired = vec![running("web", "nginx:alpine"), absent("worker"), running("cache", "redis:7")];
        let observed = vec![
            observed("web", "nginx:alpine", ObservedStatus::Running), // converged
            observed("worker", "busybox", ObservedStatus::Running),   // needs removal
            // "cache" never observed -> needs deploy
        ];
        let actions = diff(&desired, &observed);
        assert_eq!(
            actions,
            vec![Action::Remove("worker".into()), Action::Deploy(desired[2].clone())]
        );
    }
}
