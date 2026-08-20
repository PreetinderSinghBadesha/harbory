use axum::{extract::State, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::store::Store;

/// Minimal JSON status endpoint — a stand-in for the real dashboard (Phase
/// 5) and its not-yet-chosen frontend stack. Read-only, unauthenticated
/// (fine for now: nothing sensitive beyond what §3's revocation UI will
/// need proper auth for anyway).
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
    Router::new().route("/agents", get(list_agents)).with_state(state)
}

async fn list_agents(State(state): State<AppState>) -> Result<Json<Vec<AgentSummaryDto>>, axum::http::StatusCode> {
    let agents = state
        .store
        .list_agents(state.online_threshold_seconds)
        .await
        .map_err(|err| {
            tracing::error!(?err, "failed to list agents");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
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
