mod accounts;
mod agents;
mod audit;
mod containers;
mod pairing;
mod proxy;
mod registration;

pub use agents::AgentRecord;
pub use audit::{AuditEventRecord, AuditEventType};
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
}
