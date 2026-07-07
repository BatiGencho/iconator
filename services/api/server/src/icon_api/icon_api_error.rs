use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct IconApiError {
    #[serde(skip)]
    pub(crate) status_code: axum::http::StatusCode,
    pub(crate) message: String,
    pub(crate) details: Vec<Detail>,
    pub(crate) timestamp: String,
}

impl IconApiError {
    pub fn bad_request(message: String, details: Vec<Detail>) -> Self {
        Self {
            status_code: axum::http::StatusCode::BAD_REQUEST,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Detail {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) field: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) code: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) message: String,
}

impl axum::response::IntoResponse for IconApiError {
    fn into_response(self) -> axum::response::Response {
        if self.status_code.is_server_error() {
            sentry::Hub::with_active(|hub| hub.capture_error(&self));
        }

        (self.status_code, axum::Json(self)).into_response()
    }
}

impl std::fmt::Display for IconApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}, {}",
            self.status_code,
            self.message,
            self.details
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}
impl std::fmt::Debug for IconApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("status_code", &self.status_code)
            .field("message", &self.message)
            .field("details", &self.details)
            .field("timestamp", &self.timestamp)
            .finish()
    }
}

impl std::error::Error for IconApiError {}
