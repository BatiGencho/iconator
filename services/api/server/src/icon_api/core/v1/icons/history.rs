//! Recent icon-lookup history, read from `icons.query_history`.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use postgres::connection::{WithConnectionError, with_connection};
use postgres::models::query_history::QueryHistory;
use types::{HistoryResponse, IconQueryEntry};

use crate::AppState;
use crate::icon_api::error_recorder::ErrorRecorder;
use crate::shared::extractors::request_id::RequestId;

use super::errors::{self, HandlerResult};

const HANDLER_NAME: &str = "icons_history";
const HISTORY_LIMIT: i64 = 10;

/// Get the last 10 DB-backed icon lookups, most recent first.
#[utoipa::path(
    get,
    path = "/icons/history",
    responses(
        (status = 200, description = "Last 10 lookups", body = HistoryResponse),
        (status = 500, description = "Internal server error"),
    ),
    tag = "icons",
)]
#[tracing::instrument(skip_all, name = "icons_history")]
pub async fn handler(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
) -> HandlerResult<(StatusCode, Json<HistoryResponse>)> {
    let recorder =
        ErrorRecorder::new(&state.telemetry, HANDLER_NAME, &request_id);

    let entries =
        with_connection(&state.read_only_pool, |mut conn| async move {
            QueryHistory::get_latest(HISTORY_LIMIT, &mut conn).await
        })
        .await
        .map_err(|e| match e {
            WithConnectionError::Pool(e) => recorder
                .record("pool_error", errors::Error::PoolError(e.to_string())),
            WithConnectionError::Operation(e) => recorder
                .record("database_error", errors::Error::DatabaseError(e)),
        })?;

    let queries = entries
        .into_iter()
        .map(|e| IconQueryEntry {
            id: e.id,
            query_kind: e.query_kind,
            query_path: e.query_path,
            icon_id: e.icon_id,
            created_at: e.created_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(HistoryResponse { queries })))
}
