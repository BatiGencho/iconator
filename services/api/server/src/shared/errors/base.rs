use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
/// Unified error structure for all APIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub status_code: u16,
    pub code: &'static str,
    pub message: String,
    pub details: Vec<ErrorDetail>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ErrorContext>,
}

/// Structured error detail for field-level errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub field: Option<String>,
    pub code: &'static str,
    pub message: String,
}

/// Additional error context for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional: Option<Value>,
}

impl ApiError {
    pub fn new(
        status_code: StatusCode,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status_code: status_code.as_u16(),
            code,
            message: message.into(),
            details: Vec::new(),
            timestamp: Utc::now().to_rfc3339(),
            context: None,
        }
    }

    pub fn with_detail(
        mut self,
        field: Option<String>,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        self.details.push(ErrorDetail {
            field,
            code,
            message: message.into(),
        });
        self
    }

    pub fn with_details(mut self, details: Vec<ErrorDetail>) -> Self {
        self.details.extend(details);
        self
    }

    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.status_code)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn report_if_server_error(&self) {
        if self.status_code >= 500 {
            sentry::capture_message(&self.message, sentry::Level::Error);
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        self.report_if_server_error();

        let status = self.status_code();
        let body = Json(json!({
            "error": self
        }));

        (status, body).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

/// Standard error codes
pub mod codes {
    pub const BAD_REQUEST: &str = "BAD_REQUEST";
    pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
    pub const FORBIDDEN: &str = "FORBIDDEN";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const CONFLICT: &str = "CONFLICT";
    pub const UNPROCESSABLE_ENTITY: &str = "UNPROCESSABLE_ENTITY";
    pub const TOO_MANY_REQUESTS: &str = "TOO_MANY_REQUESTS";

    pub const INTERNAL_SERVER_ERROR: &str = "INTERNAL_SERVER_ERROR";
    pub const BAD_GATEWAY: &str = "BAD_GATEWAY";
    pub const SERVICE_UNAVAILABLE: &str = "SERVICE_UNAVAILABLE";
    pub const GATEWAY_TIMEOUT: &str = "GATEWAY_TIMEOUT";

    pub const DATABASE_ERROR: &str = "DATABASE_ERROR";
    pub const VALIDATION_ERROR: &str = "VALIDATION_ERROR";
    pub const INVALID_UUID: &str = "INVALID_UUID";
    pub const INSUFFICIENT_BALANCE: &str = "INSUFFICIENT_BALANCE";
    pub const EXPIRED_TOKEN: &str = "EXPIRED_TOKEN";
}
