use diesel::pg::Pg;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::task;
use tokio_postgres::Client as TokioPgClient;
use tracing::Instrument;
use tracing::{info, instrument, warn};

pub type Pool = bb8::Pool<AsyncPgConnection>;
pub type PooledConnection = bb8::PooledConnection<'static, AsyncPgConnection>;

pub const MAX_POOL_SIZE: u32 = 300;
pub const MIN_RESERVED_CONNECTIONS: u32 = 10;

pub async fn create_tokio_pg_client(
    db_url: &str,
) -> Result<TokioPgClient, tokio_postgres::Error> {
    let (client, connection) =
        tokio_postgres::connect(db_url, tokio_postgres::NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("PostgreSQL connection error: {}", e);
        }
    });

    Ok(client)
}

pub async fn get_max_connections(
    client: &TokioPgClient,
) -> Result<i32, anyhow::Error> {
    let row = client
        .query_one("SELECT current_setting('max_connections')", &[])
        .await?;

    let max_conn_str: String = row.get(0);
    let max_conn: i32 = max_conn_str.parse().map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse max_connections '{}': {}",
            max_conn_str,
            e
        )
    })?;

    Ok(max_conn)
}

fn calculate_optimal_pool_size(
    db_max_connections: i32,
    num_app_instances: u32,
    reserved_for_admin: u32,
) -> u32 {
    let available =
        db_max_connections.saturating_sub(reserved_for_admin as i32);

    let per_instance =
        (available as f32 / num_app_instances as f32).floor() as u32;

    per_instance.min(MAX_POOL_SIZE)
}

pub async fn establish_connection(
    db_url: String,
) -> Result<Pool, anyhow::Error> {
    let client = create_tokio_pg_client(&db_url).await.map_err(|e| {
        anyhow::anyhow!("Failed to create PostgreSQL tokio client: {}", e)
    })?;

    let max_conn = get_max_connections(&client).await?;
    info!("PostgreSQL max_connections: {}", max_conn);

    let max_pool_size =
        calculate_optimal_pool_size(max_conn, 1, MIN_RESERVED_CONNECTIONS);
    info!("PostgreSQL max_pool_size: {}", max_pool_size);

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_url);
    let pool = bb8::Pool::builder()
        .max_size(max_pool_size)
        .connection_timeout(Duration::from_secs(10))
        .idle_timeout(Some(Duration::from_secs(180)))
        .retry_connection(true)
        .max_lifetime(Some(Duration::from_secs(3600)))
        .build(config)
        .await?;

    let mut conn = pool.get_owned().await?;
    diesel::sql_query("SELECT 1").execute(&mut conn).await?;

    Ok(pool)
}

/// Max size for a pool built via [`build_lazy_pool`]. Kept modest because such a
/// pool only exists when the database was unreachable at startup (degraded mode).
pub const DEFAULT_LAZY_POOL_SIZE: u32 = 10;

/// Build a connection pool *without* establishing or validating any connection.
///
/// Used for resilient startup: if Postgres is unreachable at boot, the server can
/// still come up and serve endpoints that don't need the database, while
/// DB-backed endpoints return a pool error (surfaced as `503`) until the database
/// becomes reachable. Connections are attempted lazily on first use, with a short
/// timeout so those requests fail fast rather than hang.
pub fn build_lazy_pool(db_url: String) -> Pool {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_url);
    bb8::Pool::builder()
        .max_size(DEFAULT_LAZY_POOL_SIZE)
        .connection_timeout(Duration::from_secs(3))
        .retry_connection(true)
        .build_unchecked(config)
}

#[instrument(skip(pool))]
pub async fn shutdown_pool_with_timeout(
    pool: Arc<Pool>,
    shutdown_timeout: Duration,
) -> Result<(), String> {
    info!("Starting graceful PostgreSQL pool shutdown with timeout");

    let state = pool.state();
    info!(
        "Current pool state - total: {}, idle: {}, active: {}",
        state.connections,
        state.idle_connections,
        state.connections - state.idle_connections
    );

    if state.connections > state.idle_connections {
        let active_count = state.connections - state.idle_connections;
        warn!(
            "Waiting for {} active database connections to finish",
            active_count
        );

        let start = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;

            let current_state = pool.state();
            let active =
                current_state.connections - current_state.idle_connections;

            if active == 0 {
                info!("All database connections are now idle");
                break;
            }

            if start.elapsed() > shutdown_timeout {
                warn!(
                    "Shutdown timeout reached with {} active connections remaining",
                    active
                );
                break;
            }
        }
    }

    drop(pool);

    info!("PostgreSQL pool shutdown complete");
    Ok(())
}

pub async fn run_migrations<A>(
    async_connection: A,
    migrations: EmbeddedMigrations,
) -> Result<(), Box<dyn Error>>
where
    A: AsyncConnection<Backend = Pg> + 'static,
{
    let mut async_wrapper: AsyncConnectionWrapper<A> =
        AsyncConnectionWrapper::from(async_connection);

    if tokio::runtime::Handle::try_current().is_err() {
        return Err(
            "This function must be called from within a Tokio runtime".into()
        );
    }

    task::spawn_blocking(move || {
        async_wrapper
            .run_pending_migrations(migrations)
            .expect("failed to run migrations");
    })
    .await?;

    Ok(())
}

/// Execute a database operation with a scoped connection.
///
/// The connection is acquired from the pool only when this function is called
/// and automatically returned to the pool when the operation completes.
/// This allows handlers to avoid holding connections for their entire lifecycle,
/// improving connection pool utilization under high concurrency.
///
/// # Example
///
/// ```rust,ignore
/// use postgres::connection::with_connection;
///
/// let result = with_connection(&state.pool, |mut conn| async move {
///     User::create(&new_user, &mut conn).await
/// }).await.map_err(|e| /* handle pool connection error */)?;
/// ```
///
/// # Tracing
///
/// This function automatically instruments connection acquisition and usage:
/// - `acquiring_pooled_connection` span: Shows time waiting for a connection from the pool
/// - `holding_db_connection` span: Shows time the connection is held and used
/// - Debug log: Connection returned to pool
///
/// # Performance Considerations
///
/// While this pattern requires acquiring a connection for each database operation
/// (rather than once per request), it significantly improves throughput under high
/// concurrency by releasing connections immediately after use. The pool acquisition
/// overhead (~1-5ms when no contention) is negligible compared to the benefits.
///
/// # Error Handling
///
/// This function returns a Result with either:
/// - `Ok(T)` - The successful result from the operation
/// - `Err(WithConnectionError<E>)` - Either a pool error or an operation error
pub async fn with_connection<F, Fut, T, E>(
    pool: &Pool,
    operation: F,
) -> Result<T, WithConnectionError<E>>
where
    F: FnOnce(PooledConnection) -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let pool_state_before = pool.state();
    let acquire_span = tracing::info_span!(
        "acquiring_pooled_connection",
        pool.connections = pool_state_before.connections,
        pool.idle_connections = pool_state_before.idle_connections,
    );

    let conn =
        async { pool.get_owned().await.map_err(WithConnectionError::Pool) }
            .instrument(acquire_span)
            .await?;

    let hold_span = tracing::info_span!("holding_db_connection");
    let result = async {
        operation(conn)
            .await
            .map_err(WithConnectionError::Operation)
    }
    .instrument(hold_span)
    .await;

    let pool_state_after = pool.state();
    tracing::debug!(
        pool.connections = pool_state_after.connections,
        pool.idle_connections = pool_state_after.idle_connections,
        "connection_returned_to_pool"
    );

    result
}

/// Error type for with_connection that distinguishes between pool and operation errors
#[derive(Debug)]
pub enum WithConnectionError<E> {
    /// Error acquiring connection from the pool
    Pool(diesel_async::pooled_connection::bb8::RunError),
    /// Error from the database operation itself
    Operation(E),
}

impl<E: std::fmt::Display> std::fmt::Display for WithConnectionError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WithConnectionError::Pool(e) => {
                write!(f, "Failed to acquire connection: {}", e)
            }
            WithConnectionError::Operation(e) => {
                write!(f, "Database operation failed: {}", e)
            }
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error
    for WithConnectionError<E>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WithConnectionError::Pool(e) => Some(e),
            WithConnectionError::Operation(e) => Some(e),
        }
    }
}

/// Helper to convert WithConnectionError<diesel::result::Error> to diesel::result::Error
///
/// This is useful when you want to use the `?` operator directly with with_connection results
/// in contexts where diesel::result::Error is expected.
///
/// # Example
///
/// ```rust,ignore
/// with_connection(&state.pool, |mut conn| async move {
///     User::create(&new_user, &mut conn).await
/// })
/// .await
/// .map_err(connection_error_to_diesel)?;
/// ```
pub fn connection_error_to_diesel(
    err: WithConnectionError<diesel::result::Error>,
) -> diesel::result::Error {
    match err {
        WithConnectionError::Pool(e) => diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UnableToSendCommand,
            Box::new(e.to_string()),
        ),
        WithConnectionError::Operation(e) => e,
    }
}

/// Execute database operations within an atomic transaction.
///
/// The transaction is acquired from the pool and all operations are executed atomically.
/// If any operation fails, all changes are automatically rolled back.
/// If all operations succeed, the transaction is automatically committed.
///
/// # Tracing
///
/// This function automatically instruments:
/// - Connection acquisition (via with_connection)
/// - Transaction execution with automatic commit/rollback logging
///
/// # Error Handling
///
/// Returns `WithConnectionError<E>` which distinguishes between:
/// - Pool errors (failed to acquire connection)
/// - Transaction errors (operation failed, transaction rolled back)
pub async fn with_transaction<F, T, E>(
    pool: &Pool,
    operation: F,
) -> Result<T, WithConnectionError<E>>
where
    F: for<'c> FnOnce(
            &'c mut AsyncPgConnection,
        ) -> futures::future::BoxFuture<'c, Result<T, E>>
        + Send,
    T: Send,
    E: From<diesel::result::Error> + std::error::Error + Send,
{
    with_connection(pool, |mut conn| async move {
        let txn_span = tracing::info_span!("database_transaction");

        async {
            let result = conn
                .transaction::<T, E, _>(|txn_conn| {
                    Box::pin(operation(txn_conn))
                })
                .await;

            match &result {
                Ok(_) => tracing::debug!("transaction_committed"),
                Err(e) => {
                    tracing::error!(error = %e, "transaction_rolled_back")
                }
            }

            result
        }
        .instrument(txn_span)
        .await
    })
    .await
}

/// Execute database operations within an atomic transaction, converting errors to diesel errors.
///
/// This is a convenience wrapper around `with_transaction` that automatically converts
/// `WithConnectionError<diesel::result::Error>` to `diesel::result::Error`.
///
pub async fn with_diesel_transaction<F, T>(
    pool: &Pool,
    operation: F,
) -> Result<T, diesel::result::Error>
where
    F: for<'c> FnOnce(
            &'c mut AsyncPgConnection,
        ) -> futures::future::BoxFuture<
            'c,
            Result<T, diesel::result::Error>,
        > + Send,
    T: Send,
{
    with_transaction(pool, operation)
        .await
        .map_err(connection_error_to_diesel)
}
