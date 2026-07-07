use crate::icon_api::icon_api_error::{Detail, IconApiError};
use crate::shared::errors::{ApiError, ErrorDetail};
use axum::http::StatusCode;

/// Compatibility layer for Icon API - converts unified ApiError to IconApiError format
impl From<ApiError> for IconApiError {
    fn from(api_error: ApiError) -> Self {
        let details: Vec<Detail> = api_error
            .details
            .into_iter()
            .map(|detail| Detail {
                field: detail.field.unwrap_or_default(),
                code: detail.code.to_string(),
                message: detail.message,
            })
            .collect();

        IconApiError {
            status_code: StatusCode::from_u16(api_error.status_code)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            message: api_error.message,
            details,
            timestamp: api_error.timestamp,
        }
    }
}

/// Conversion from IconApiError to unified ApiError
impl From<IconApiError> for ApiError {
    fn from(icon_api_error: IconApiError) -> Self {
        let details: Vec<ErrorDetail> = icon_api_error
            .details
            .into_iter()
            .map(|detail| ErrorDetail {
                field: if detail.field.is_empty() {
                    None
                } else {
                    Some(detail.field)
                },
                code: Box::leak(detail.code.into_boxed_str()), // Convert String to &'static str for compatibility
                message: detail.message,
            })
            .collect();

        ApiError {
            status_code: icon_api_error.status_code.as_u16(),
            code: "ICONAPIERROR",
            message: icon_api_error.message,
            details,
            timestamp: icon_api_error.timestamp,
            context: None,
        }
    }
}
