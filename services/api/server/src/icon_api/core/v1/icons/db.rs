//! DB-backed icon lookups: read through Redis, fall back to Postgres, and log
//! each lookup to `icons.query_history`. The in-memory variant lives in `memory.rs`.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use deadpool_redis::redis::AsyncCommands;
use postgres::connection::{WithConnectionError, with_connection};
use postgres::models::query_history::{NewQueryHistory, QueryHistory};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::icon_api::error_recorder::ErrorRecorder;
use crate::shared::extractors::request_id::RequestId;

use super::errors::{self, HandlerResult, empty_path_error};
use super::models::{IconQuery, IconResponse, IconSource};
use super::resolve::{Target, resolve_db};

const CACHE_TTL_SECONDS: u64 = 300; // 5 minutes

/// What we store in Redis. Misses (`icon_id: None`) are cached too, so unknown
/// paths don't keep hitting Postgres.
#[derive(Serialize, Deserialize)]
struct CachedIcon {
    icon_id: Option<i64>,
}

/// Resolve the icon for a file path (exact name, then extension).
#[utoipa::path(
    get,
    path = "/icons/file",
    params(IconQuery),
    responses(
        (status = 200, description = "Icon lookup result", body = IconResponse),
        (status = 400, description = "Missing or empty path"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "icons",
)]
#[tracing::instrument(skip_all, name = "icons_db_file")]
pub async fn file(
    state: State<AppState>,
    request_id: RequestId,
    query: Query<IconQuery>,
) -> HandlerResult<(StatusCode, Json<IconResponse>)> {
    handle(state, request_id, Target::File, query.0.path).await
}

/// Resolve the icon for a folder path (exact folder name).
#[utoipa::path(
    get,
    path = "/icons/folder",
    params(IconQuery),
    responses(
        (status = 200, description = "Icon lookup result", body = IconResponse),
        (status = 400, description = "Missing or empty path"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "icons",
)]
#[tracing::instrument(skip_all, name = "icons_db_folder")]
pub async fn folder(
    state: State<AppState>,
    request_id: RequestId,
    query: Query<IconQuery>,
) -> HandlerResult<(StatusCode, Json<IconResponse>)> {
    handle(state, request_id, Target::Folder, query.0.path).await
}

async fn handle(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    target: Target,
    path: String,
) -> HandlerResult<(StatusCode, Json<IconResponse>)> {
    if path.trim().is_empty() {
        return Err(empty_path_error(&request_id));
    }

    tracing::info!(
        kind = target.as_str(),
        path = %path,
        request_id = %request_id,
        "icon lookup (db)",
    );

    let handler_name = match target {
        Target::File => "icons_db_file",
        Target::Folder => "icons_db_folder",
    };
    let recorder =
        ErrorRecorder::new(&state.telemetry, handler_name, &request_id);

    let cache_key = format!("icon:db:{}:{}", target.as_str(), path);

    // Try the cache first. A miss or a Redis error just falls through to the DB.
    let mut icon_id = None;
    let mut cache_hit = false;
    if let Ok(mut conn) = state.cache_pool.get().await {
        let cached: Result<Option<String>, _> = conn.get(&cache_key).await;
        if let Ok(Some(json_str)) = cached
            && let Ok(entry) = serde_json::from_str::<CachedIcon>(&json_str)
        {
            icon_id = entry.icon_id;
            cache_hit = true;
            tracing::debug!("cache hit for {cache_key}");
        }
    }

    // Cache miss: resolve against the read pool, then backfill the cache.
    if !cache_hit {
        let path_for_db = path.clone();
        icon_id =
            with_connection(&state.read_only_pool, |mut conn| async move {
                resolve_db(target, &path_for_db, &mut conn).await
            })
            .await
            .map_err(|e| map_conn_err(&recorder, e))?;

        if let Ok(json_str) = serde_json::to_string(&CachedIcon { icon_id })
            && let Ok(mut conn) = state.cache_pool.get().await
        {
            let _: Result<(), _> =
                conn.set_ex(&cache_key, &json_str, CACHE_TTL_SECONDS).await;
        }
    }

    // Log the lookup. Synchronous for now; if this ever shows up on the read
    // path's latency, move it off-thread or batch it.
    let entry = NewQueryHistory {
        query_kind: target.as_str().to_string(),
        query_path: path.clone(),
        icon_id,
    };
    with_connection(&state.pool, |mut conn| async move {
        QueryHistory::create(entry, &mut conn).await
    })
    .await
    .map_err(|e| map_conn_err(&recorder, e))?;

    Ok((
        StatusCode::OK,
        Json(IconResponse {
            path,
            icon_id,
            found: icon_id.is_some(),
            source: IconSource::Database,
        }),
    ))
}

fn map_conn_err(
    recorder: &ErrorRecorder<'_>,
    e: WithConnectionError<diesel::result::Error>,
) -> crate::icon_api::icon_api_error_v1::IconApiV1Error {
    match e {
        WithConnectionError::Pool(e) => recorder
            .record("pool_error", errors::Error::PoolError(e.to_string())),
        WithConnectionError::Operation(e) => {
            recorder.record("database_error", errors::Error::DatabaseError(e))
        }
    }
}
