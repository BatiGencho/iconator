use crate::AppState;
use crate::shared::extractors::error::Error;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use chrono::Utc;
use redis::connection::PooledConnection;
use tracing::Instrument;

pub struct CacheConnection(pub PooledConnection);

impl FromRequestParts<AppState> for CacheConnection {
    type Rejection = Error;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let conn = state
            .cache_pool
            .get()
            .instrument(tracing::info_span!("acquiring_cache_connection"))
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
