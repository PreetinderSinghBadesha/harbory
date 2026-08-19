mod agents;
mod audit;
mod pairing;
mod registration;

pub use agents::AgentRecord;
pub use audit::AuditEventType;
pub use pairing::IssuedPairingToken;
pub use registration::{RegisterError, RegisterOutcome, VerifyCredentialError};

use sqlx::PgPool;

/// Thin wrapper around the Postgres pool. Kept as a single struct (rather
/// than a trait with an in-memory test double) since Phase 1 tests run
/// against a real database — see /docs/database.md.
#[derive(Clone)]
pub struct Store {
    pub(crate) pool: PgPool,
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    /// Minimal account creation. Full account/auth management is Phase 5;
    /// this exists so Phase 1 has something to hang pairing tokens and
    /// agents off of.
    pub async fn create_account(&self, email: &str) -> Result<uuid::Uuid, sqlx::Error> {
        let rec = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO accounts (email) VALUES ($1) RETURNING id",
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;
        Ok(rec)
    }
}
