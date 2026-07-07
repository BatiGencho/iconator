//! In-memory icon lookups, straight from `iconator`'s fst maps - no Postgres, no
//! Redis. The data is baked into the binary at build time, so a lookup is a single
//! fst probe. Kept next to the DB handlers so the two approaches can be compared.

use axum::Json;
use axum::extract::Query;
use axum::http::StatusCode;

use crate::shared::extractors::request_id::RequestId;

use super::errors::{HandlerResult, empty_path_error};
use super::models::{IconQuery, IconResponse, IconSource};
use super::resolve::{Target, resolve_memory};

/// Resolve the icon for a file path from the in-memory maps.
#[utoipa::path(
    get,
    path = "/icons/memory/file",
    params(IconQuery),
    responses(
        (status = 200, description = "Icon lookup result", body = IconResponse),
        (status = 400, description = "Missing or empty path"),
    ),
    tag = "icons",
)]
#[tracing::instrument(skip_all, name = "icons_mem_file")]
pub async fn file(
    request_id: RequestId,
    query: Query<IconQuery>,
) -> HandlerResult<(StatusCode, Json<IconResponse>)> {
    handle(request_id, Target::File, query.0.path)
}

/// Resolve the icon for a folder path from the in-memory maps.
#[utoipa::path(
    get,
    path = "/icons/memory/folder",
    params(IconQuery),
    responses(
        (status = 200, description = "Icon lookup result", body = IconResponse),
        (status = 400, description = "Missing or empty path"),
    ),
    tag = "icons",
)]
#[tracing::instrument(skip_all, name = "icons_mem_folder")]
pub async fn folder(
    request_id: RequestId,
    query: Query<IconQuery>,
) -> HandlerResult<(StatusCode, Json<IconResponse>)> {
    handle(request_id, Target::Folder, query.0.path)
}

fn handle(
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
        "icon lookup (memory)",
    );

    let icon_id = resolve_memory(target, &path);

    Ok((
        StatusCode::OK,
        Json(IconResponse {
            path,
            icon_id,
            found: icon_id.is_some(),
            source: IconSource::Memory,
        }),
    ))
}
