use sqlx::{PgPool, Result, FromRow};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct DesiredComposeStack {
    pub agent_id: Uuid,
    pub name: String,
    pub repo_url: String,
    pub git_ref: String,
    pub compose_file_path: String,
    pub desired_status: String,
}

#[derive(Debug, FromRow)]
pub struct ObservedComposeStack {
    pub agent_id: Uuid,
    pub name: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct ComposeStore {
    pool: PgPool,
}

impl ComposeStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_desired(&self, stack: &DesiredComposeStack) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO desired_compose_stacks (agent_id, name, repo_url, git_ref, compose_file_path, desired_status)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (agent_id, name) DO UPDATE SET
                repo_url = EXCLUDED.repo_url,
                git_ref = EXCLUDED.git_ref,
                compose_file_path = EXCLUDED.compose_file_path,
                desired_status = EXCLUDED.desired_status,
                updated_at = now()
            "#,
        )
        .bind(stack.agent_id)
        .bind(&stack.name)
        .bind(&stack.repo_url)
        .bind(&stack.git_ref)
        .bind(&stack.compose_file_path)
        .bind(&stack.desired_status)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_desired_by_agent(&self, agent_id: Uuid) -> Result<Vec<DesiredComposeStack>> {
        sqlx::query_as::<_, DesiredComposeStack>(
            r#"
            SELECT agent_id, name, repo_url, git_ref, compose_file_path, desired_status
            FROM desired_compose_stacks
            WHERE agent_id = $1
            "#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn replace_observed(&self, agent_id: Uuid, stacks: &[ObservedComposeStack]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM observed_compose_stacks WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;

        for stack in stacks {
            sqlx::query(
                r#"
                INSERT INTO observed_compose_stacks (agent_id, name, status, error)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(agent_id)
            .bind(&stack.name)
            .bind(&stack.status)
            .bind(&stack.error)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_observed_by_agent(&self, agent_id: Uuid) -> Result<Vec<ObservedComposeStack>> {
        sqlx::query_as::<_, ObservedComposeStack>(
            r#"
            SELECT agent_id, name, status, error
            FROM observed_compose_stacks
            WHERE agent_id = $1
            "#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
    }
}
