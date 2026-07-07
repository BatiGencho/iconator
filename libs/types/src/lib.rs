//! Shared API types for the Icon Lookup API - the contract between the server
//! (`icon-api`) and the client (`sdk`). Defining them once here keeps request
//! and response shapes from drifting between the two.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Where a lookup result was served from.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum IconSource {
    /// Resolved against Postgres (with a Redis cache in front).
    Database,
    /// Resolved against the in-memory fst maps from `iconator`.
    Memory,
}

/// Result of an icon lookup. `iconId` is null (and `found` is false) when no
/// icon matches - a normal outcome (the client falls back to a default icon),
/// so this is returned with `200 OK` rather than `404`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IconResponse {
    #[schema(example = "./src/main.rs")]
    pub path: String,
    #[schema(example = 525)]
    pub icon_id: Option<i64>,
    pub found: bool,
    pub source: IconSource,
}

/// A single recorded lookup from the query history.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IconQueryEntry {
    pub id: Uuid,
    /// 'file' or 'folder'.
    pub query_kind: String,
    #[schema(example = "./src/main.rs")]
    pub query_path: String,
    #[schema(example = 525)]
    pub icon_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// Response containing the most recent lookups.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResponse {
    pub queries: Vec<IconQueryEntry>,
}
