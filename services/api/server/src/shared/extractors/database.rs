use crate::AppState;
use crate::shared::extractors::error::Error;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use chrono::Utc;
use postgres::connection::PooledConnection;
use tracing::Instrument;

pub struct DatabaseConnection(pub PooledConnection);

impl FromRequestParts<AppState> for DatabaseConnection {
    type Rejection = Error;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let conn = state
            .pool
            .get_owned()
            .instrument(tracing::info_span!("acquiring_pooled_connection"))
            .await
            .map_err(internal_error)?;

        Ok(Self(conn))
    }
}

pub struct ReadOnlyDatabaseConnection(pub PooledConnection);

impl FromRequestParts<AppState> for ReadOnlyDatabaseConnection {
    type Rejection = Error;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // pool state check
        let pool_state = state.pool.state();
        if pool_state.idle_connections == 0 {
            tracing::warn!(
                "Pool low on connections - idle: {}, total: {}",
                pool_state.idle_connections,
                pool_state.connections,
            );
        }

        let conn = state
            .read_only_pool
            .get_owned()
            .instrument(tracing::info_span!("acquiring_pooled_connection"))
            .await
            .map_err(internal_error)?;

        Ok(Self(conn))
    }
}

fn internal_error<E>(err: E) -> Error
where
    E: std::error::Error,
{
    Error {
        status_code: StatusCode::INTERNAL_SERVER_ERROR,
        code: "INTERNAL_SERVER_ERROR",
        message: err.to_string(),
        timestamp: Utc::now().naive_utc().to_string(),
        custom: Default::default(),
    }
}
