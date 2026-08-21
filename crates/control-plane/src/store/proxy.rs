use harbory_protocol::v1::ProxyRoute;
use uuid::Uuid;

use super::Store;

// Reuses the wire type directly rather than a parallel domain struct —
// unlike containers, there's no extra concept (no desired_status enum)
// beyond what's already on the wire for this resource, so an intermediate
// type would just be duplication.

#[derive(sqlx::FromRow)]
struct ProxyRouteRow {
    name: String,
    server_name: String,
    listen_port: i32,
    path_prefix: String,
    upstream_host: String,
    upstream_port: i32,
}

impl ProxyRouteRow {
    fn into_proto(self) -> ProxyRoute {
        ProxyRoute {
            name: self.name,
            server_name: self.server_name,
            listen_port: self.listen_port as u32,
            path_prefix: self.path_prefix,
            upstream_host: self.upstream_host,
            upstream_port: self.upstream_port as u32,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ProxyStateRow {
    applied_hash: Vec<u8>,
    error: Option<String>,
}

impl Store {
    pub async fn upsert_desired_proxy_route(&self, agent_id: Uuid, route: &ProxyRoute) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO desired_proxy_routes
                (agent_id, name, server_name, listen_port, path_prefix, upstream_host, upstream_port, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, now())
             ON CONFLICT (agent_id, name) DO UPDATE
             SET server_name = $3, listen_port = $4, path_prefix = $5, upstream_host = $6,
                 upstream_port = $7, updated_at = now()",
        )
        .bind(agent_id)
        .bind(&route.name)
        .bind(&route.server_name)
        .bind(route.listen_port as i32)
        .bind(&route.path_prefix)
        .bind(&route.upstream_host)
        .bind(route.upstream_port as i32)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Unlike `set_desired_absent` for containers, this really does delete
    /// the row — there's no "absent" status to flip to, see the module
    /// doc comment. Returns `false` if nothing by that name existed.
    pub async fn delete_desired_proxy_route(&self, agent_id: Uuid, name: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM desired_proxy_routes WHERE agent_id = $1 AND name = $2")
            .bind(agent_id)
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_desired_proxy_routes(&self, agent_id: Uuid) -> Result<Vec<ProxyRoute>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ProxyRouteRow>(
            "SELECT name, server_name, listen_port, path_prefix, upstream_host, upstream_port
             FROM desired_proxy_routes WHERE agent_id = $1 ORDER BY name",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(ProxyRouteRow::into_proto).collect())
    }

    pub async fn record_proxy_state(
        &self,
        agent_id: Uuid,
        applied_hash: &[u8],
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO proxy_state (agent_id, applied_hash, error, reported_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (agent_id) DO UPDATE SET applied_hash = $2, error = $3, reported_at = now()",
        )
        .bind(agent_id)
        .bind(applied_hash)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_proxy_state(&self, agent_id: Uuid) -> Result<Option<(Vec<u8>, Option<String>)>, sqlx::Error> {
        let row = sqlx::query_as::<_, ProxyStateRow>("SELECT applied_hash, error FROM proxy_state WHERE agent_id = $1")
            .bind(agent_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| (r.applied_hash, r.error)))
    }
}
