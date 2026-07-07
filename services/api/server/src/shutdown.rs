use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;
use tokio::sync::{Mutex, Notify};
use tokio::time::Duration;
use tracing::{info, warn};

pub struct ShutdownCoordinator {
    notify: Arc<Notify>,
    shutting_down: AtomicBool,
    inner: Mutex<Option<ShutdownInner>>,
}

struct ShutdownInner {
    db_pool: postgres::connection::Pool,
    redis_pool: redis::connection::Pool,
}

impl ShutdownCoordinator {
    pub fn new(
        db_pool: postgres::connection::Pool,
        redis_pool: redis::connection::Pool,
    ) -> Self {
        Self {
            notify: Arc::new(Notify::new()),
            shutting_down: AtomicBool::new(false),
            inner: Mutex::new(Some(ShutdownInner {
                db_pool,
                redis_pool,
            })),
        }
    }

    pub async fn wait_for_shutdown(&self) {
        self.notify.notified().await;
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::Relaxed)
    }

    pub async fn shutdown(&self) {
        self.shutting_down.store(true, Ordering::Relaxed);
        info!("Initiating graceful shutdown sequence");

        let inner = match self.inner.lock().await.take() {
            Some(inner) => inner,
            None => {
                warn!("Shutdown already called");
                return;
            }
        };

        self.notify.notify_waiters();
        let shutdown_timeout = Duration::from_secs(10);

        let db_handle = tokio::spawn({
            let pool = inner.db_pool.clone();
            async move {
                match tokio::time::timeout(
                    shutdown_timeout,
                    postgres::connection::shutdown_pool_with_timeout(
                        pool.into(),
                        shutdown_timeout,
                    ),
                )
                .await
                {
                    Ok(Ok(_)) => info!("Database pool shutdown completed"),
                    Ok(Err(e)) => {
                        warn!("Database pool shutdown error: {:?}", e)
                    }
                    Err(_) => warn!("Database pool shutdown timed out"),
                }
            }
        });

        let redis_handle = tokio::spawn({
            let pool = inner.redis_pool.clone();
            async move {
                match tokio::time::timeout(
                    shutdown_timeout,
                    redis::connection::shutdown_pool_with_timeout(
                        pool,
                        shutdown_timeout,
                    ),
                )
                .await
                {
                    Ok(_) => info!("Redis pool shutdown completed"),
                    Err(_) => warn!("Redis pool shutdown timed out"),
                }
            }
        });

        let _ = tokio::join!(db_handle, redis_handle);

        info!("Graceful shutdown sequence complete");
    }
}

pub async fn listen_for_shutdown_signals() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        }
        _ = terminate => {
            info!("Received SIGTERM signal");
        }
    }

    info!("signal received, starting graceful shutdown");
}
