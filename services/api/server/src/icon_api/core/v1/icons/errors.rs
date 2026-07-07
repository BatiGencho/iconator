use uuid::Uuid;

use crate::icon_api::icon_api_error_v1::{IconApiV1Detail, IconApiV1Error};

pub type HandlerResult<T> = Result<T, IconApiV1Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),

    #[error("Failed to get database connection: {0}")]
    PoolError(String),
}

impl Error {
    // Consumes `self` by design (mirrors the `IntoIconApiV1Error` conversion below).
    #[allow(clippy::wrong_self_convention)]
    pub fn to_icon_api_v1_error(self, request_id: &Uuid) -> IconApiV1Error {
        match self {
            Error::DatabaseError(e) => IconApiV1Error::internal_server_error(
                "Icon lookup failed".to_string(),
                vec![IconApiV1Detail {
                    field: None,
                    code: "database_error".to_string(),
                    message: format!("Database error: {e}"),
                    suggestion: "Please try again later".to_string(),
                    documentation: String::new(),
                }],
                request_id.to_string(),
            ),
            Error::PoolError(e) => IconApiV1Error::service_unavailable(
                "Service temporarily unavailable".to_string(),
                vec![IconApiV1Detail {
                    field: None,
                    code: "pool_error".to_string(),
                    message: format!("Failed to get database connection: {e}"),
                    suggestion: "Please try again later".to_string(),
                    documentation: String::new(),
                }],
                request_id.to_string(),
            ),
        }
    }
}

impl crate::icon_api::error_recorder::IntoIconApiV1Error for Error {
    fn into_icon_api_v1_error(self, request_id: &Uuid) -> IconApiV1Error {
        self.to_icon_api_v1_error(request_id)
    }
}

/// Reject an empty `path` query parameter with `400 Bad Request`.
pub fn empty_path_error(request_id: &Uuid) -> IconApiV1Error {
    IconApiV1Error::bad_request(
        "Invalid request parameters".to_string(),
        vec![IconApiV1Detail {
            field: Some("path".to_string()),
            code: "missing_path".to_string(),
            message: "The `path` query parameter must not be empty".to_string(),
            suggestion: "Provide a path, e.g. ?path=./src/main.rs".to_string(),
            documentation: String::new(),
        }],
        request_id.to_string(),
    )
}
