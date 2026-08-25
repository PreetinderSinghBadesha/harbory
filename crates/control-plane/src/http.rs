use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Redirect,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use harbory_protocol::v1::ProxyRoute;
use metrics_exporter_prometheus::PrometheusHandle;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::auth::AuthenticatedAccount;
use crate::github;
use crate::jwks::JwkVerifier;
use crate::reconcile::{DesiredContainer, DesiredStatus, GitSource, ObservedContainer, ObservedStatus, PortMapping};
use crate::store::{AuditEventType, Store};
use crate::stream::ConnectionRegistry;

/// JSON API behind the dashboard — every route requires a valid Supabase
/// JWT (`AuthenticatedAccount`, see auth.rs) and, for anything scoped to a
/// specific agent, ownership of that agent. Not yet a full REST API for
/// third-party use; this is what `frontend/` talks to.
#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub online_threshold_seconds: i64,
    /// The legacy shared HS256 secret, if configured — verifies older-style
    /// Supabase tokens. See `jwt_secret` alongside `jwks` in `auth.rs`.
    pub jwt_secret: Option<String>,
    /// Supabase's asymmetric (ES256) signing keys, if a project URL was
    /// configured — verifies newer-style Supabase tokens. See `auth.rs`.
    pub jwks: JwkVerifier,
    pub metrics_handle: PrometheusHandle,
    /// Live agent connections — used to forward log snapshot requests into
    /// the persistent gRPC stream without a separate connection or RPC.
    pub registry: ConnectionRegistry,
    /// GitHub OAuth App credentials + where to send the browser back to
    /// after the OAuth round trip — all `None`/absent when the GitHub
    /// integration isn't configured, which every handler that needs them
    /// treats as "not available" (503) rather than the control plane
    /// refusing to start, since this is an optional feature unlike the
    /// fail-fast Supabase JWT check.
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub github_redirect_uri: Option<String>,
    pub frontend_url: Option<String>,
}

/// Confirms `agent_id` exists *and* belongs to `account` in one check —
/// 404 (not 403) either way, so a caller probing agent ids they don't own
/// can't distinguish "doesn't exist" from "exists but isn't yours".
async fn require_owned_agent(state: &AppState, account: &AuthenticatedAccount, agent_id: Uuid) -> Result<(), StatusCode> {
    let agent = state
        .store
        .get_agent(agent_id)
        .await
        .map_err(|err| {
            tracing::error!(?err, "failed to look up agent for ownership check");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if agent.account_id != account.id {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(())
}

pub fn router(state: AppState) -> Router {
    Router::new()
        // Deliberately unauthenticated: metrics are process-global
        // aggregates (no per-account/per-agent data), and scrape
        // endpoints are conventionally protected at the network layer
        // (firewall/internal-only routing), not per-request auth — see
        // docs/observability.md.
        .route("/metrics", get(metrics_endpoint))
        .route("/me", get(me))
        .route("/pairing-tokens", post(create_pairing_token))
        .route("/agents", get(list_agents))
        .route("/security-events", get(list_security_events))
        .route("/agents/:agent_id/revoke", post(revoke_agent))
        .route("/agents/:agent_id", delete(delete_agent_handler))
        .route("/agents/:agent_id/containers", get(list_containers))
        .route("/agents/:agent_id/containers/:name", put(put_container).delete(delete_container))
        .route("/agents/:agent_id/containers/:name/logs", get(get_container_logs))
        .route("/agents/:agent_id/images", get(list_images))
        .route("/agents/:agent_id/images/:image_id", delete(delete_image))
        .route("/agents/:agent_id/proxy-routes", get(list_proxy_routes))
        .route("/agents/:agent_id/proxy-routes/:name", put(put_proxy_route).delete(delete_proxy_route))
        .route("/agents/:agent_id/compose-stacks", get(list_compose_stacks))
        .route("/agents/:agent_id/compose-stacks/:name", put(put_compose_stack).delete(delete_compose_stack))
        .route("/github/oauth/start", post(github_oauth_start))
        // Deliberately unauthenticated, like /metrics above: this is hit
        // by a real browser redirect from github.com, which can't carry
        // our normal Authorization header — see github_oauth_callback's
        // doc comment for how it recovers which account this is for.
        .route("/github/oauth/callback", get(github_oauth_callback))
        .route("/github/repos", get(list_github_repos))
        .route("/github/connection", delete(delete_github_connection))
        // Bearer-token auth (not cookies), so a permissive CORS policy
        // doesn't carry the usual credentialed-CORS/CSRF risk — a
        // cross-origin page can't make the browser attach a token it
        // doesn't already have.
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn metrics_endpoint(State(state): State<AppState>) -> String {
    state.metrics_handle.render()
}

#[derive(Serialize)]
struct MeDto {
    id: Uuid,
    email: Option<String>,
}

async fn me(account: AuthenticatedAccount) -> Json<MeDto> {
    Json(MeDto { id: account.id, email: account.email })
}

#[derive(Deserialize)]
struct CreatePairingTokenRequest {
    #[serde(default = "default_pairing_token_ttl_minutes")]
    ttl_minutes: i64,
}

fn default_pairing_token_ttl_minutes() -> i64 {
    10
}

#[derive(Serialize)]
struct PairingTokenDto {
    token: String,
    expires_at: DateTime<Utc>,
}

/// Issues a fresh pairing token for the authenticated account — the
/// backend for the "agent pairing UI" (generate/display a token + install
/// command). Replaces the dev-only `examples/issue_token.rs` CLI for
/// anything account-scoped; that example still exists for quick local
/// testing without a Supabase login.
async fn create_pairing_token(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    Json(req): Json<CreatePairingTokenRequest>,
) -> Result<Json<PairingTokenDto>, StatusCode> {
    let ttl = Duration::minutes(req.ttl_minutes.clamp(1, 60));
    let issued = state.store.issue_pairing_token(account.id, ttl).await.map_err(|err| {
        tracing::error!(?err, "failed to issue pairing token");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(PairingTokenDto { token: issued.plaintext, expires_at: issued.expires_at }))
}

#[derive(Serialize)]
struct SecurityEventDto {
    event_type: String,
    agent_id: Option<Uuid>,
    detail: serde_json::Value,
    created_at: DateTime<Utc>,
    is_misuse_signal: bool,
}

const SECURITY_EVENTS_LIMIT: i64 = 100;

/// The dashboard's activity feed — in-dashboard alerting for misuse
/// signals (pairing token reuse, credential fingerprint mismatch) rather
/// than email, since no external email service is wired up yet. See
/// docs/observability.md.
async fn list_security_events(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
) -> Result<Json<Vec<SecurityEventDto>>, StatusCode> {
    let events =
        state.store.list_audit_events_for_account(account.id, SECURITY_EVENTS_LIMIT).await.map_err(|err| {
            tracing::error!(?err, "failed to list security events");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(
        events
            .into_iter()
            .map(|e| SecurityEventDto {
                event_type: e.event_type,
                agent_id: e.agent_id,
                detail: e.detail,
                created_at: e.created_at,
                is_misuse_signal: e.is_misuse_signal,
            })
            .collect(),
    ))
}

#[derive(Serialize)]
struct AgentSummaryDto {
    id: Uuid,
    account_id: Uuid,
    status: String,
    online: bool,
    last_heartbeat_at: Option<DateTime<Utc>>,
}

async fn list_agents(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
) -> Result<Json<Vec<AgentSummaryDto>>, StatusCode> {
    let agents =
        state.store.list_agents_for_account(account.id, state.online_threshold_seconds).await.map_err(|err| {
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

/// Per §3: revoked agents can only rejoin via a brand-new pairing token —
/// this is the operator action that starts that. 404 if the agent doesn't
/// exist / isn't yours; a second revoke on an already-revoked agent is
/// also a 404 (`Store::revoke_agent` only flips `active -> revoked`, and
/// "already not active" isn't distinguished from "never existed" for the
/// same don't-leak-existence reason as `require_owned_agent`).
async fn revoke_agent(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    Path(agent_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

    let revoked = state.store.revoke_agent(agent_id).await.map_err(|err| {
        tracing::error!(?err, "failed to revoke agent");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if revoked {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Permanently deletes the agent and everything scoped to it (desired/
/// observed state, proxy routes, compose stacks — see
/// `Store::delete_agent` for the full list). Also kicks any live stream
/// immediately so the deleted agent stops receiving reconcile commands;
/// it can never reconnect, since its credential no longer verifies.
/// 404 if the agent doesn't exist / isn't yours.
async fn delete_agent_handler(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    Path(agent_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

    let deleted = state.store.delete_agent(agent_id).await.map_err(|err| {
        tracing::error!(?err, "failed to delete agent");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if deleted.is_some() {
        state.registry.kick(agent_id).await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize)]
struct PortMappingDto {
    host_port: u16,
    container_port: u16,
}

#[derive(Deserialize)]
struct GitSourceDto {
    repo_url: String,
    #[serde(default)]
    git_ref: String,
    #[serde(default)]
    dockerfile_path: String,
}

#[derive(Deserialize)]
struct PutContainerRequest {
    /// Required unless `source` is given — mutually exclusive with it.
    #[serde(default)]
    image: String,
    #[serde(default)]
    env: Vec<String>,
    #[serde(default)]
    ports: Vec<PortMappingDto>,
    #[serde(default)]
    command: Vec<String>,
    #[serde(default)]
    source: Option<GitSourceDto>,
}

/// Declares desired state for one container (create it if new, update it
/// in place if it already exists) — takes effect the next time the agent
/// reports its state, up to one heartbeat interval later, not instantly.
/// See docs/reconciliation.md for why that latency is an accepted
/// trade-off for now rather than a bug.
async fn put_container(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    Path((agent_id, name)): Path<(Uuid, String)>,
    Json(req): Json<PutContainerRequest>,
) -> Result<StatusCode, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

    // `image` is a real pull reference for a plain deploy, or — when
    // `source` is given — computed here as the synthetic
    // "git+<repo>#<ref>" identity string `reconcile::diff` uses to decide
    // whether to redeploy. Never both; a request needs exactly one.
    let (image, git_source) = match req.source {
        Some(src) if !src.repo_url.is_empty() => {
            let image = format!("git+{}#{}", src.repo_url, src.git_ref);
            (image, Some(GitSource { repo_url: src.repo_url, git_ref: src.git_ref, dockerfile_path: src.dockerfile_path }))
        }
        Some(_) => return Err(StatusCode::BAD_REQUEST),
        None if !req.image.is_empty() => (req.image, None),
        None => return Err(StatusCode::BAD_REQUEST),
    };

    let container = DesiredContainer {
        name,
        image,
        env: req.env,
        ports: req.ports.into_iter().map(|p| PortMapping { host_port: p.host_port, container_port: p.container_port }).collect(),
        command: req.command,
        status: DesiredStatus::Running,
        git_source,
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
    account: AuthenticatedAccount,
    Path((agent_id, name)): Path<(Uuid, String)>,
) -> Result<StatusCode, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

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
struct GitSourceOutDto {
    repo_url: String,
    git_ref: String,
    dockerfile_path: String,
}

#[derive(Serialize)]
struct DesiredContainerDto {
    name: String,
    image: String,
    env: Vec<String>,
    ports: Vec<PortMappingDto2>,
    command: Vec<String>,
    status: &'static str,
    source: Option<GitSourceOutDto>,
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
    error: Option<String>,
}

#[derive(Serialize)]
struct ContainersDto {
    desired: Vec<DesiredContainerDto>,
    observed: Vec<ObservedContainerDto>,
}

async fn list_containers(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<ContainersDto>, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

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
                source: d.git_source.map(|g| GitSourceOutDto { repo_url: g.repo_url, git_ref: g.git_ref, dockerfile_path: g.dockerfile_path }),
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
                error: o.error,
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
struct PutComposeStackRequest {
    repo_url: String,
    #[serde(default)]
    git_ref: String,
    #[serde(default = "default_compose_file_path")]
    compose_file_path: String,
    #[serde(default)]
    env: Vec<String>,
}

fn default_compose_file_path() -> String {
    "docker-compose.yml".to_string()
}

#[derive(Serialize)]
struct ComposeStackOutDto {
    name: String,
    repo_url: String,
    git_ref: String,
    compose_file_path: String,
    env: Vec<String>,
    status: &'static str,
}

#[derive(Serialize)]
struct ObservedComposeStackDto {
    name: String,
    status: &'static str,
    error: Option<String>,
}

#[derive(Serialize)]
struct ComposeStacksDto {
    desired: Vec<ComposeStackOutDto>,
    observed: Vec<ObservedComposeStackDto>,
}

async fn put_compose_stack(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    Path((agent_id, name)): Path<(Uuid, String)>,
    Json(req): Json<PutComposeStackRequest>,
) -> Result<StatusCode, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

    let repo_url = req.repo_url;

    let stack = crate::store::DesiredComposeStack {
        agent_id,
        name: name.clone(),
        repo_url,
        git_ref: req.git_ref,
        compose_file_path: req.compose_file_path,
        env: req.env,
        desired_status: "running".to_string(),
    };

    state.store.compose.upsert_desired(&stack).await.map_err(|err| {
        tracing::error!(?err, "failed to upsert compose stack");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // We don't have AuditEventType::PutComposeStack yet, using a dummy or omitting.
    // state.store.log_audit_event(agent_id, AuditEventType::PutComposeStack, &name).await.ok();
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_compose_stack(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    Path((agent_id, name)): Path<(Uuid, String)>,
) -> Result<StatusCode, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

    let stack = crate::store::DesiredComposeStack {
        agent_id,
        name: name.clone(),
        repo_url: "".into(),
        git_ref: "".into(),
        compose_file_path: "".into(),
        env: vec![],
        desired_status: "absent".to_string(),
    };

    state.store.compose.upsert_desired(&stack).await.map_err(|err| {
        tracing::error!(?err, "failed to update compose stack to absent");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(StatusCode::NO_CONTENT)
}

async fn list_compose_stacks(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<ComposeStacksDto>, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

    let desired = state.store.compose.get_desired_by_agent(agent_id).await.unwrap_or_default();
    let observed = state.store.compose.get_observed_by_agent(agent_id).await.unwrap_or_default();

    Ok(Json(ComposeStacksDto {
        desired: desired.into_iter().map(|d| ComposeStackOutDto {
            name: d.name,
            repo_url: d.repo_url,
            git_ref: d.git_ref,
            compose_file_path: d.compose_file_path,
            env: d.env,
            status: if d.desired_status == "absent" { "absent" } else { "running" },
        }).collect(),
        observed: observed.into_iter().map(|o| ObservedComposeStackDto {
            name: o.name,
            status: match o.status.as_str() {
                "running" => "running",
                "stopped" => "stopped",
                "removed" => "removed",
                "error" => "error",
                _ => "error",
            },
            error: o.error,
        }).collect(),
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
    account: AuthenticatedAccount,
    Path((agent_id, name)): Path<(Uuid, String)>,
    Json(req): Json<PutProxyRouteRequest>,
) -> Result<StatusCode, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

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
    account: AuthenticatedAccount,
    Path((agent_id, name)): Path<(Uuid, String)>,
) -> Result<StatusCode, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

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
    account: AuthenticatedAccount,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<ProxyRoutesDto>, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

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

// --- GitHub OAuth App integration -----------------------------------------
//
// v1 scope: connect a GitHub account and list its repos. Deploying a
// container *from* one of those repos is a separate, later piece of work
// — this alone is already independently useful/testable end to end.

fn github_configured(state: &AppState) -> Option<(&str, &str, &str)> {
    Some((state.github_client_id.as_deref()?, state.github_client_secret.as_deref()?, state.github_redirect_uri.as_deref()?))
}

#[derive(Serialize)]
struct OAuthStartDto {
    authorize_url: String,
}

const GITHUB_OAUTH_STATE_TTL_MINUTES: i64 = 10;

/// Step one of the OAuth Authorization Code flow. Returns a URL for the
/// frontend to navigate the browser to directly (`window.location.href =
/// ...`, not a fetch) — GitHub's own redirect back to
/// `github_oauth_callback` is what completes the round trip.
async fn github_oauth_start(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
) -> Result<Json<OAuthStartDto>, StatusCode> {
    let Some((client_id, _secret, redirect_uri)) = github_configured(&state) else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let issued = state
        .store
        .issue_github_oauth_state(account.id, Duration::minutes(GITHUB_OAUTH_STATE_TTL_MINUTES))
        .await
        .map_err(|err| {
            tracing::error!(?err, "failed to issue github oauth state");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(OAuthStartDto { authorize_url: github::oauth_authorize_url(client_id, redirect_uri, &issued.plaintext) }))
}

#[derive(Deserialize)]
struct GitHubCallbackParams {
    code: Option<String>,
    state: Option<String>,
    /// Set instead of `code` if the user declined on GitHub's consent
    /// screen — not an error worth logging, just "they said no".
    error: Option<String>,
}

/// Step two: GitHub redirects the browser here directly, so this can't
/// require the normal `Authorization` header the rest of the API uses —
/// `state` (looked up via `consume_github_oauth_state`, the same
/// row-locked single-use pattern `register_agent` uses for pairing
/// tokens) is what recovers which account started the flow. Always
/// redirects back into the dashboard rather than rendering a raw error
/// to the browser, since a person is looking at this, not a script.
async fn github_oauth_callback(State(state): State<AppState>, Query(params): Query<GitHubCallbackParams>) -> Result<Redirect, StatusCode> {
    let Some(frontend_url) = state.frontend_url.as_deref() else {
        // Nowhere to send them back to — this is a real misconfiguration,
        // not a normal failure mode, so it's the one case that doesn't
        // redirect.
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };
    let error_redirect = Ok(Redirect::to(&format!("{frontend_url}/dashboard?github=error")));

    if params.error.is_some() {
        return error_redirect;
    }
    let (Some(code), Some(state_param)) = (params.code, params.state) else {
        return error_redirect;
    };
    let Some((client_id, client_secret, redirect_uri)) = github_configured(&state) else {
        return error_redirect;
    };

    let account_id = match state.store.consume_github_oauth_state(&state_param).await {
        Ok(id) => id,
        Err(err) => {
            tracing::warn!(?err, "github oauth callback with invalid/expired/reused state");
            return error_redirect;
        }
    };

    let access_token = match github::exchange_code(client_id, client_secret, &code, redirect_uri).await {
        Ok(token) => token,
        Err(err) => {
            tracing::error!(?err, "github code exchange failed");
            return error_redirect;
        }
    };
    let github_login = match github::fetch_login(&access_token).await {
        Ok(login) => login,
        Err(err) => {
            tracing::error!(?err, "failed to fetch github user after successful code exchange");
            return error_redirect;
        }
    };

    if let Err(err) = state.store.upsert_github_connection(account_id, &access_token, &github_login).await {
        tracing::error!(?err, "failed to store github connection");
        return error_redirect;
    }
    let _ = state
        .store
        .record_audit_event(AuditEventType::GitHubConnected, Some(account_id), None, serde_json::json!({ "github_login": github_login }))
        .await;

    Ok(Redirect::to(&format!("{frontend_url}/dashboard?github=connected")))
}

#[derive(Serialize)]
struct GitHubRepoDto {
    full_name: String,
    private: bool,
    default_branch: String,
    html_url: String,
}

#[derive(Serialize)]
struct GitHubReposDto {
    github_login: String,
    repos: Vec<GitHubRepoDto>,
}

/// 404 doubles as "not connected yet" — same don't-invent-a-new-shape
/// convention as everywhere else `require_owned_agent` uses 404 for "not
/// yours/doesn't exist".
async fn list_github_repos(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
) -> Result<Json<GitHubReposDto>, StatusCode> {
    let connection = state.store.get_github_connection(account.id).await.map_err(|err| {
        tracing::error!(?err, "failed to load github connection");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let Some(connection) = connection else {
        return Err(StatusCode::NOT_FOUND);
    };

    let repos = github::list_repos(&connection.access_token).await.map_err(|err| {
        tracing::error!(?err, "failed to list github repos");
        StatusCode::BAD_GATEWAY
    })?;

    Ok(Json(GitHubReposDto {
        github_login: connection.github_login,
        repos: repos
            .into_iter()
            .map(|r| GitHubRepoDto { full_name: r.full_name, private: r.private, default_branch: r.default_branch, html_url: r.html_url })
            .collect(),
    }))
}

async fn delete_github_connection(State(state): State<AppState>, account: AuthenticatedAccount) -> Result<StatusCode, StatusCode> {
    let deleted = state.store.delete_github_connection(account.id).await.map_err(|err| {
        tracing::error!(?err, "failed to delete github connection");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if deleted {
        let _ = state.store.record_audit_event(AuditEventType::GitHubDisconnected, Some(account.id), None, serde_json::json!({})).await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize)]
struct LogsQuery {
    #[serde(default)]
    tail: u32,
}

#[derive(Serialize)]
struct ContainerLogsDto {
    logs: String,
    error: String,
}

/// Fetches a snapshot of recent container logs by forwarding a `LogsRequest`
/// over the agent's live gRPC stream and awaiting the `LogsResponse`.
///
/// 503 — agent has no live connection (offline or not yet connected)
/// 504 — agent connected but didn't respond within 5 seconds
async fn get_container_logs(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    Path((agent_id, container_name)): Path<(Uuid, String)>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<ContainerLogsDto>, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

    let request_id = Uuid::new_v4().to_string();
    let tail = if query.tail == 0 { 100 } else { query.tail };

    let rx = state
        .registry
        .request_logs(agent_id, request_id, container_name, tail)
        .await
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?; // agent offline

    match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
        Ok(Ok(resp)) => Ok(Json(ContainerLogsDto { logs: resp.logs, error: resp.error })),
        Ok(Err(_)) => {
            // oneshot sender was dropped (connection died between sending and replying)
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
        Err(_) => {
            // 5-second timeout elapsed
            Err(StatusCode::GATEWAY_TIMEOUT)
        }
    }
}

#[derive(Serialize)]
struct ImageInfoDto {
    id: String,
    repo_tags: Vec<String>,
    size_bytes: i64,
    created_at: i64,
    in_use: bool,
}

#[derive(Serialize)]
struct ImagesDto {
    images: Vec<ImageInfoDto>,
    // Populated when the agent failed to list, or a removal was requested
    // and failed — the caller gets whatever fresh listing exists plus the
    // reason, instead of a bare status code with no detail.
    error: String,
}

/// Forwards an images request over the agent's live stream — list-only for
/// GET, remove-then-list for DELETE (the refreshed list doubles as the
/// delete's response so the UI updates in one round trip).
///
/// 503 — agent has no live connection; 504 — no reply within 5 seconds.
async fn images_via_stream(
    State(state): State<AppState>,
    account: AuthenticatedAccount,
    agent_id: Uuid,
    remove_image_id: String,
) -> Result<Json<ImagesDto>, StatusCode> {
    require_owned_agent(&state, &account, agent_id).await?;

    let request_id = Uuid::new_v4().to_string();
    let rx = state
        .registry
        .request_images(agent_id, request_id, remove_image_id)
        .await
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
        Ok(Ok(resp)) => Ok(Json(ImagesDto {
            error: resp.error,
            images: resp
                .images
                .into_iter()
                .map(|i| ImageInfoDto {
                    id: i.id,
                    repo_tags: i.repo_tags,
                    size_bytes: i.size_bytes,
                    created_at: i.created_at,
                    in_use: i.in_use,
                })
                .collect(),
        })),
        Ok(Err(_)) => Err(StatusCode::SERVICE_UNAVAILABLE),
        Err(_) => Err(StatusCode::GATEWAY_TIMEOUT),
    }
}

async fn list_images(
    state: State<AppState>,
    account: AuthenticatedAccount,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<ImagesDto>, StatusCode> {
    images_via_stream(state, account, agent_id, String::new()).await
}

async fn delete_image(
    state: State<AppState>,
    account: AuthenticatedAccount,
    Path((agent_id, image_id)): Path<(Uuid, String)>,
) -> Result<Json<ImagesDto>, StatusCode> {
    images_via_stream(state, account, agent_id, image_id).await
}

