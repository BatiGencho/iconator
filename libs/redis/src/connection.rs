use deadpool_redis::Runtime;
use deadpool_redis::redis::AsyncCommands;
use std::{sync::Arc, time::Duration};
use tracing::{info, instrument};

pub type Pool = Arc<deadpool_redis::Pool>;
pub type PooledConnection = deadpool_redis::Connection;

pub async fn establish_connection(
    redis_url: String,
) -> Result<Pool, anyhow::Error> {
    let pool = build_lazy_pool(redis_url)?;

    // Validate connectivity up-front so a healthy startup fails loudly on misconfig.
    let mut conn = pool.get().await?;
    let _: () = conn.ping().await?;

    Ok(pool)
}

/// Build the Redis pool without validating connectivity (deadpool connects
/// lazily on first use). Used for resilient startup: if Redis is unreachable at
/// boot, the server still comes up and lookups simply run uncached until it
/// recovers. Fails only on an invalid URL / config, which is a genuine error.
pub fn build_lazy_pool(redis_url: String) -> Result<Pool, anyhow::Error> {
    let mut cfg = deadpool_redis::Config::from_url(redis_url);
    cfg.pool = Some(deadpool_redis::PoolConfig {
        max_size: 50,
        ..Default::default()
    });
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

    Ok(Arc::new(pool))
}

#[instrument(skip(pool))]
pub async fn shutdown_pool_with_timeout(pool: Pool, timeout: Duration) {
    info!("Starting graceful Redis connection pool shutdown");

    tokio::time::sleep(Duration::from_millis(100)).await;
    pool.close();

    info!("Redis connection pool closed");
}
