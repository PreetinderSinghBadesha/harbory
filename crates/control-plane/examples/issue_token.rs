//! Dev-only helper: create a scratch account and print a pairing token for
//! it. Stand-in for the dashboard UI that Phase 5 will build.
//! Usage: cargo run -p harbory-control-plane --example issue_token

use chrono::Duration;
use harbory_control_plane::store::Store;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://harbory:harbory_dev_password@localhost:55433/harbory".into()
    });
    let store = Store::connect(&database_url).await?;

    let email = format!("dev-{}@example.test", uuid::Uuid::new_v4());
    let account_id = store.create_account(&email).await?;
    let token = store.issue_pairing_token(account_id, Duration::minutes(10)).await?;

    println!("{}", token.plaintext);
    Ok(())
}
