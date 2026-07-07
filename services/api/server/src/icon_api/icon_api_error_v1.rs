use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct IconApiV1Error {
    #[serde(skip)]
    pub(crate) status_code: axum::http::StatusCode,
    pub(crate) message: String,
    pub(crate) details: Vec<IconApiV1Detail>,
    pub(crate) timestamp: String,
    pub(crate) request_id: String,
}

impl IconApiV1Error {
    pub fn bad_request(
        message: String,
        details: Vec<IconApiV1Detail>,
        request_id: String,
    ) -> Self {
        Self {
            status_code: axum::http::StatusCode::BAD_REQUEST,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
            request_id,
        }
    }

    pub fn forbidden(
        message: String,
        details: Vec<IconApiV1Detail>,
        request_id: String,
    ) -> Self {
        Self {
            status_code: axum::http::StatusCode::FORBIDDEN,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
            request_id,
        }
    }

    pub fn not_found(
        message: String,
        details: Vec<IconApiV1Detail>,
        request_id: String,
    ) -> Self {
        Self {
            status_code: axum::http::StatusCode::NOT_FOUND,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
            request_id,
        }
    }

    pub fn internal_server_error(
        message: String,
        details: Vec<IconApiV1Detail>,
        request_id: String,
    ) -> Self {
        Self {
            status_code: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
            request_id,
        }
    }

    pub fn service_unavailable(
        message: String,
        details: Vec<IconApiV1Detail>,
        request_id: String,
    ) -> Self {
        Self {
            status_code: axum::http::StatusCode::SERVICE_UNAVAILABLE,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
            request_id,
        }
    }

    pub fn unauthorized(
        message: String,
        details: Vec<IconApiV1Detail>,
        request_id: String,
    ) -> Self {
        Self {
            status_code: axum::http::StatusCode::UNAUTHORIZED,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
            request_id,
        }
    }

    pub fn unprocessable_entity(
        message: String,
        details: Vec<IconApiV1Detail>,
        request_id: String,
    ) -> Self {
        Self {
            status_code: axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
            request_id,
        }
    }

    pub fn too_many_requests(
        message: String,
        details: Vec<IconApiV1Detail>,
        request_id: String,
    ) -> Self {
        Self {
            status_code: axum::http::StatusCode::TOO_MANY_REQUESTS,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
            request_id,
        }
    }

    pub fn bad_gateway(
        message: String,
        details: Vec<IconApiV1Detail>,
        request_id: String,
    ) -> Self {
        Self {
            status_code: axum::http::StatusCode::BAD_GATEWAY,
            message,
            details,
            timestamp: Utc::now().to_rfc3339(),
            request_id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct IconApiV1Detail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) field: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) code: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) message: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) suggestion: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) documentation: String,
}

impl axum::response::IntoResponse for IconApiV1Error {
    fn into_response(self) -> axum::response::Response {
        if self.status_code.is_server_error() {
            sentry::Hub::with_active(|hub| hub.capture_error(&self));
        }

        (self.status_code, axum::Json(self)).into_response()
    }
}

impl std::fmt::Display for IconApiV1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}, {}, {}",
            self.status_code,
            self.message,
            self.request_id,
            self.details
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}
impl std::fmt::Debug for IconApiV1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("status_code", &self.status_code)
            .field("message", &self.message)
            .field("details", &self.details)
            .field("timestamp", &self.timestamp)
            .field("request_id", &self.request_id)
            .finish()
    }
}

impl std::error::Error for IconApiV1Error {}
