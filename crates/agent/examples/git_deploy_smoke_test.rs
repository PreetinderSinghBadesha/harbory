//! Dev-only: exercises the git-sourced deploy path end to end against a
//! real Docker daemon, without the control plane in the loop — deploys a
//! container built from a real repo, then confirms `list_state()` reports
//! back exactly the identity string the spec was given (not Docker's own
//! locally-built tag). That equality is the entire point of the
//! `harbory.image` label fix in `container.rs`: it's what would let
//! `reconcile::diff` on the control-plane side see this as converged and
//! *not* redeploy (rebuild) on every subsequent heartbeat.
//!
//! Usage:
//!   cargo run -p harbory-agent --example git_deploy_smoke_test -- \
//!     <repo_url> [git_ref] [dockerfile_path]
//!
//! Needs a real Docker daemon reachable via the usual local socket/pipe —
//! this only ever ran reviewed-but-unexercised on this Windows dev
//! machine (no Docker here), same situation `docs/proxy-management.md`
//! already documents for the nginx side of the agent. Run this the first
//! time on an actual Linux host with Docker installed.

use harbory_agent::container::ContainerManager;
use harbory_protocol::v1::{ContainerSpec, ContainerStatus, GitSource};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut args = std::env::args().skip(1);
    let repo_url = args.next().expect("usage: git_deploy_smoke_test <repo_url> [git_ref] [dockerfile_path]");
    let git_ref = args.next().unwrap_or_default();
    let dockerfile_path = args.next().unwrap_or_default();

    let name = "git-smoke-test".to_string();
    let image = format!("git+{repo_url}#{git_ref}");

    let spec = ContainerSpec {
        name: name.clone(),
        image: image.clone(),
        env: vec![],
        ports: vec![],
        command: vec![],
        git_source: Some(GitSource { repo_url, git_ref, dockerfile_path }),
    };

    let containers = ContainerManager::connect()?;

    println!("deploying...");
    containers.deploy(&spec).await?;
    println!("deployed. reported state:");

    let states = containers.list_state().await?;
    let observed = states.iter().find(|s| s.name == name).expect("just-deployed container missing from list_state");
    println!("  name:   {}", observed.name);
    println!("  image:  {}", observed.image);
    println!("  status: {:?}", ContainerStatus::try_from(observed.status).unwrap_or(ContainerStatus::Unspecified));

    assert_eq!(
        observed.image, image,
        "reported image doesn't match the spec's identity string — reconciliation would never converge"
    );
    println!("\nOK: reported image matches the spec's identity string exactly.");

    containers.remove(&name).await;
    println!("cleaned up.");

    Ok(())
}
