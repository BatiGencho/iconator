use super::base::{ApiError, codes};
use super::common::{database_error, invalid_uuid, validation_error};

impl ApiError {
    pub fn from_database_error(error: diesel::result::Error) -> ApiError {
        match error {
            diesel::result::Error::NotFound => ApiError::new(
                axum::http::StatusCode::NOT_FOUND,
                codes::NOT_FOUND,
                "Resource not found",
            ),
            diesel::result::Error::DatabaseError(kind, info) => match kind {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    ApiError::new(
                        axum::http::StatusCode::CONFLICT,
                        codes::CONFLICT,
                        format!(
                            "Unique constraint violation: {}",
                            info.message()
                        ),
                    )
                }
                diesel::result::DatabaseErrorKind::ForeignKeyViolation => {
                    ApiError::new(
                        axum::http::StatusCode::BAD_REQUEST,
                        codes::BAD_REQUEST,
                        format!(
                            "Referenced resource does not exist: {}",
                            info.message()
                        ),
                    )
                }
                diesel::result::DatabaseErrorKind::CheckViolation => {
                    ApiError::new(
                        axum::http::StatusCode::BAD_REQUEST,
                        codes::VALIDATION_ERROR,
                        format!("Data validation failed: {}", info.message()),
                    )
                }
                _ => database_error(format!("{:?}: {}", kind, info.message())),
            },
            _ => database_error(error),
        }
    }

    pub fn from_validation_error(
        error: validator::ValidationErrors,
    ) -> ApiError {
        let mut api_error = validation_error("Validation failed");

        for (field, field_errors) in error.field_errors() {
            for field_error in field_errors {
                let message = field_error
                    .message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| {
                        format!("Invalid value for field '{}'", field)
                    });

                api_error = api_error.with_detail(
                    Some(field.to_string()),
                    codes::VALIDATION_ERROR,
                    message,
                );
            }
        }

        api_error
    }
}

// Standard From implementations for common error types
impl From<diesel::result::Error> for ApiError {
    fn from(error: diesel::result::Error) -> Self {
        Self::from_database_error(error)
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(error: validator::ValidationErrors) -> Self {
        Self::from_validation_error(error)
    }
}

impl From<uuid::Error> for ApiError {
    fn from(error: uuid::Error) -> Self {
        invalid_uuid(format!("{}", error))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            codes::BAD_REQUEST,
            format!("JSON parsing error: {}", error),
        )
    }
}

impl From<axum::extract::rejection::JsonRejection> for ApiError {
    fn from(error: axum::extract::rejection::JsonRejection) -> Self {
        ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            codes::BAD_REQUEST,
            format!("Invalid JSON payload: {}", error),
        )
    }
}
