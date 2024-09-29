use std::time::Duration;

use anyhow::Context;
use backoff::ExponentialBackoff;
use sqlx::PgPool;

use crate::{configuration::DatabaseSettings, startup::get_connection_pool};

pub async fn run_worker_until_stopped(settings: DatabaseSettings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&settings);
    worker_loop(connection_pool).await
}

async fn worker_loop(pool: PgPool) -> Result<(), anyhow::Error> {
    loop {
        let _ = try_delete_expired_keys(&pool).await;
        tokio::time::sleep(Duration::from_secs(60 * 60 * 12)).await;
    }
}

#[tracing::instrument("Delete expired idempotency keys", skip_all)]
pub async fn try_delete_expired_keys(pool: &PgPool) -> Result<(), anyhow::Error> {
    let operation = || async {
        sqlx::query("DELETE FROM idempotency WHERE created_at < NOW() - INTERVAL '1 days'")
            .execute(pool)
            .await
            .context("Cannot delete indempotency keys")?;
        Ok(())
    };
    let backoff = ExponentialBackoff::default();
    backoff::future::retry(backoff, operation).await
}
