use chrono::Utc;
use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Error {
    #[serde(skip)]
    pub status_code: axum::http::StatusCode,
    /// Easy lookup code for the error
    pub code: &'static str,
    /// Detailed description of what happened
    pub message: String,
    /// Timestamp at which the error happened
    pub timestamp: String,
    /// User-defined custom fields
    pub custom: HashMap<String, serde_json::Value>,
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        if self.status_code.is_server_error() {
            sentry::Hub::with_active(|hub| hub.capture_error(&self));
        }

        (self.status_code, axum::Json(self)).into_response()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.status_code, self.code, self.message)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("status_code", &self.status_code)
            .field("code", &self.code)
            .field("message", &self.message)
            .field("timestamp", &self.timestamp)
            .field("custom", &self.custom)
            .finish()
    }
}

impl std::error::Error for Error {}

impl Default for Error {
    fn default() -> Self {
        Self {
            status_code: Default::default(),
            code: "",
            message: "".to_string(),
            timestamp: Utc::now().naive_utc().to_string(),
            custom: Default::default(),
        }
    }
}
