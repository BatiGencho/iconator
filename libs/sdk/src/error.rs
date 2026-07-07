use serde::Deserialize;
use thiserror::Error;

pub type IconApiResult<T> = Result<T, IconApiError>;

#[derive(Error, Debug)]
pub enum IconApiError {
    #[error("HTTP request failed: {0}")]
    Middleware(#[from] reqwest_middleware::Error),

    #[error("HTTP transport error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("JSON serialization/deserialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API returned {status}: {message}")]
    Api { status: u16, message: String },
}

/// Best-effort view of the server's error body (`IconApiV1Error`). Only the fields
/// the SDK surfaces are modelled; unknown fields are ignored.
#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    #[serde(default)]
    pub message: String,
}
