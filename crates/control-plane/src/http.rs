use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use harbory_protocol::v1::ProxyRoute;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::reconcile::{DesiredContainer, DesiredStatus, ObservedContainer, ObservedStatus, PortMapping};
use crate::store::Store;

/// Minimal JSON status/control endpoint — a stand-in for the real
/// dashboard (Phase 5) and its not-yet-chosen frontend stack. Read/write,
/// unauthenticated (fine for now: nothing sensitive beyond what §3's
/// revocation UI will need proper auth for anyway).
#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub online_threshold_seconds: i64,
}

#[derive(Serialize)]
struct AgentSummaryDto {
    id: Uuid,
    account_id: Uuid,
    status: String,
    online: bool,
    last_heartbeat_at: Option<DateTime<Utc>>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/agents", get(list_agents))
        .route("/agents/:agent_id/containers", get(list_containers))
        .route("/agents/:agent_id/containers/:name", put(put_container).delete(delete_container))
        .route("/agents/:agent_id/proxy-routes", get(list_proxy_routes))
        .route("/agents/:agent_id/proxy-routes/:name", put(put_proxy_route).delete(delete_proxy_route))
        .with_state(state)
}

async fn list_agents(State(state): State<AppState>) -> Result<Json<Vec<AgentSummaryDto>>, StatusCode> {
    let agents = state.store.list_agents(state.online_threshold_seconds).await.map_err(|err| {
        tracing::error!(?err, "failed to list agents");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(
        agents
            .into_iter()
            .map(|a| AgentSummaryDto {
                id: a.id,
                account_id: a.account_id,
                status: a.status,
                online: a.online,
                last_heartbeat_at: a.last_heartbeat_at,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct PortMappingDto {
    host_port: u16,
    container_port: u16,
}

#[derive(Deserialize)]
struct PutContainerRequest {
    image: String,
    #[serde(default)]
    env: Vec<String>,
    #[serde(default)]
    ports: Vec<PortMappingDto>,
    #[serde(default)]
    command: Vec<String>,
}

/// Declares desired state for one container (create it if new, update it
/// in place if it already exists) — takes effect the next time the agent
/// reports its state, up to one heartbeat interval later, not instantly.
/// See docs/reconciliation.md for why that latency is an accepted
/// trade-off for now rather than a bug.
async fn put_container(
    State(state): State<AppState>,
    Path((agent_id, name)): Path<(Uuid, String)>,
    Json(req): Json<PutContainerRequest>,
) -> Result<StatusCode, StatusCode> {
    let container = DesiredContainer {
        name,
        image: req.image,
        env: req.env,
        ports: req.ports.into_iter().map(|p| PortMapping { host_port: p.host_port, container_port: p.container_port }).collect(),
        command: req.command,
        status: DesiredStatus::Running,
    };

    state.store.upsert_desired_container(agent_id, &container).await.map_err(|err| {
        tracing::error!(?err, "failed to upsert desired container");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Marks a container desired-absent. 404 if nothing by that name was ever
/// declared for this agent — there's no "running" declaration to retract.
async fn delete_container(
    State(state): State<AppState>,
    Path((agent_id, name)): Path<(Uuid, String)>,
) -> Result<StatusCode, StatusCode> {
    let found = state.store.set_desired_absent(agent_id, &name).await.map_err(|err| {
        tracing::error!(?err, "failed to mark desired container absent");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if found {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Serialize)]
struct DesiredContainerDto {
    name: String,
    image: String,
    env: Vec<String>,
    ports: Vec<PortMappingDto2>,
    command: Vec<String>,
    status: &'static str,
}

#[derive(Serialize)]
struct PortMappingDto2 {
    host_port: u16,
    container_port: u16,
}

#[derive(Serialize)]
struct ObservedContainerDto {
    name: String,
    image: String,
    status: &'static str,
}

#[derive(Serialize)]
struct ContainersDto {
    desired: Vec<DesiredContainerDto>,
    observed: Vec<ObservedContainerDto>,
}

async fn list_containers(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<ContainersDto>, StatusCode> {
    let desired = state.store.get_desired_containers(agent_id).await.map_err(|err| {
        tracing::error!(?err, "failed to load desired containers");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let observed = state.store.get_observed_containers(agent_id).await.map_err(|err| {
        tracing::error!(?err, "failed to load observed containers");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ContainersDto {
        desired: desired
            .into_iter()
            .map(|d| DesiredContainerDto {
                name: d.name,
                image: d.image,
                env: d.env,
                ports: d.ports.into_iter().map(|p| PortMappingDto2 { host_port: p.host_port, container_port: p.container_port }).collect(),
                command: d.command,
                status: match d.status {
                    DesiredStatus::Running => "running",
                    DesiredStatus::Absent => "absent",
                },
            })
            .collect(),
        observed: observed
            .into_iter()
            .map(|o: ObservedContainer| ObservedContainerDto {
                name: o.name,
                image: o.image,
                status: match o.status {
                    ObservedStatus::Running => "running",
                    ObservedStatus::Stopped => "stopped",
                    ObservedStatus::Removed => "removed",
                    ObservedStatus::Error => "error",
                },
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
struct PutProxyRouteRequest {
    #[serde(default)]
    server_name: String,
    listen_port: u16,
    #[serde(default = "default_path_prefix")]
    path_prefix: String,
    upstream_host: String,
    upstream_port: u16,
}

fn default_path_prefix() -> String {
    "/".to_string()
}

/// Declares (or updates) one proxy route. Like containers, takes effect
/// the next time the agent reports its proxy state, not instantly — see
/// docs/proxy-management.md.
async fn put_proxy_route(
    State(state): State<AppState>,
    Path((agent_id, name)): Path<(Uuid, String)>,
    Json(req): Json<PutProxyRouteRequest>,
) -> Result<StatusCode, StatusCode> {
    let route = ProxyRoute {
        name,
        server_name: req.server_name,
        listen_port: req.listen_port as u32,
        path_prefix: req.path_prefix,
        upstream_host: req.upstream_host,
        upstream_port: req.upstream_port as u32,
    };

    state.store.upsert_desired_proxy_route(agent_id, &route).await.map_err(|err| {
        tracing::error!(?err, "failed to upsert desired proxy route");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Removes a proxy route outright (there's no "absent" status for routes
/// — see docs/proxy-management.md). 404 if nothing by that name existed.
async fn delete_proxy_route(
    State(state): State<AppState>,
    Path((agent_id, name)): Path<(Uuid, String)>,
) -> Result<StatusCode, StatusCode> {
    let found = state.store.delete_desired_proxy_route(agent_id, &name).await.map_err(|err| {
        tracing::error!(?err, "failed to delete desired proxy route");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if found {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Serialize)]
struct ProxyRouteDto {
    name: String,
    server_name: String,
    listen_port: u32,
    path_prefix: String,
    upstream_host: String,
    upstream_port: u32,
}

#[derive(Serialize)]
struct ProxyRoutesDto {
    desired: Vec<ProxyRouteDto>,
    applied_hash: Option<String>,
    error: Option<String>,
}

async fn list_proxy_routes(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<ProxyRoutesDto>, StatusCode> {
    let desired = state.store.get_desired_proxy_routes(agent_id).await.map_err(|err| {
        tracing::error!(?err, "failed to load desired proxy routes");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let applied = state.store.get_proxy_state(agent_id).await.map_err(|err| {
        tracing::error!(?err, "failed to load proxy state");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ProxyRoutesDto {
        desired: desired
            .into_iter()
            .map(|r| ProxyRouteDto {
                name: r.name,
                server_name: r.server_name,
                listen_port: r.listen_port,
                path_prefix: r.path_prefix,
                upstream_host: r.upstream_host,
                upstream_port: r.upstream_port,
            })
            .collect(),
        applied_hash: applied.as_ref().map(|(hash, _)| hex::encode(hash)),
        error: applied.and_then(|(_, error)| error),
    }))
}
