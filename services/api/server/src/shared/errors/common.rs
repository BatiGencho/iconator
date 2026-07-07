use super::base::{ApiError, ErrorContext, codes};
use axum::http::StatusCode;
use serde_json::Value;

/// Common error constructors
pub fn internal_server_error(message: impl Into<String>) -> ApiError {
    ApiError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        codes::INTERNAL_SERVER_ERROR,
        message,
    )
}

pub fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, codes::BAD_REQUEST, message)
}

pub fn unauthorized(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, codes::UNAUTHORIZED, message)
}

pub fn forbidden(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, codes::FORBIDDEN, message)
}

pub fn not_found(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, codes::NOT_FOUND, message)
}

pub fn conflict(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, codes::CONFLICT, message)
}

pub fn unprocessable_entity(message: impl Into<String>) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        codes::UNPROCESSABLE_ENTITY,
        message,
    )
}

pub fn database_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        codes::DATABASE_ERROR,
        format!("Database operation failed: {}", error),
    )
}

pub fn invalid_uuid(uuid: impl Into<String>) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        codes::INVALID_UUID,
        format!("Invalid UUID: {}", uuid.into()),
    )
}

pub fn validation_error(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, codes::VALIDATION_ERROR, message)
}

/// Helper for creating context with additional data
pub fn with_additional_context(additional: Value) -> ErrorContext {
    ErrorContext {
        trace_id: None,
        additional: Some(additional),
    }
}
