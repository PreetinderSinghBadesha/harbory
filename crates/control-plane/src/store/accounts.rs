use uuid::Uuid;

use super::Store;

impl Store {
    /// Test/dev-tool account creation: a fresh, locally-generated id. Real
    /// (Phase 5+) accounts come from `get_or_create_account_by_id` instead
    /// — this stays because most of the existing test suite predates
    /// Supabase auth and doesn't need it, and re-plumbing every test to
    /// mint a fake Supabase JWT would couple otherwise-unrelated tests
    /// (pairing, container/proxy reconciliation) to auth infrastructure
    /// they don't exercise. See docs/dashboard.md.
    pub async fn create_account(&self, email: &str) -> Result<Uuid, sqlx::Error> {
        let rec =
            sqlx::query_scalar::<_, Uuid>("INSERT INTO accounts (email) VALUES ($1) RETURNING id")
                .bind(email)
                .fetch_one(&self.pool)
                .await?;
        Ok(rec)
    }

    /// The real path: `id` is the `sub` claim from a verified Supabase JWT
    /// (== that user's `auth.users.id`), so every account this creates
    /// corresponds to a real Supabase-authenticated identity. Idempotent —
    /// called on every authenticated request, not just "first login" —
    /// and updates the stored email if Supabase's copy has changed since.
    pub async fn get_or_create_account_by_id(&self, id: Uuid, email: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO accounts (id, email) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET email = $2",
        )
        .bind(id)
        .bind(email)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
