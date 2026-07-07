use crate::icon_api::icon_api_error_v1::{IconApiV1Detail, IconApiV1Error};
use crate::shared::extractors::error::Error as ExtractorError;
use crate::shared::extractors::payload;
use crate::shared::extractors::payload::Payload;
use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::borrow::Cow;
use thiserror::Error;
use uuid::Uuid;

/// ValidatedPayload uses the full request body and therefore should always appear after
/// other extractors that might implement FromRequestParts instead.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedPayload<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedPayload<T>
where
    T: serde::de::DeserializeOwned + validator::Validate + std::any::Any,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(
        req: Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|header| header.to_str().ok())
            .and_then(|header_str| Uuid::parse_str(header_str).ok())
            .unwrap_or_else(Uuid::new_v4);

        let Payload(value) = Payload::<T>::from_request(req, state)
            .await
            .map_err(|e| Error::PayloadWithRequestId(e, request_id))?;

        match value.validate() {
            Ok(_) => Ok(ValidatedPayload(value)),
            Err(e) => Err(Error::ValidationWithRequestId(e, request_id)),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Validation(#[from] validator::ValidationErrors),

    #[error(transparent)]
    Payload(#[from] payload::Error),

    #[error("Validation failed")]
    ValidationWithRequestId(validator::ValidationErrors, Uuid),

    #[error("Payload error")]
    PayloadWithRequestId(payload::Error, Uuid),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let err = ExtractorError::from(self);
        err.into_response()
    }
}

impl Error {
    pub fn to_icon_api_v1_error(self, request_id: &Uuid) -> IconApiV1Error {
        match self {
            Error::Validation(validation_errors) => {
                let details = validation_errors_to_icon_api_v1_details(
                    &validation_errors,
                );
                IconApiV1Error::bad_request(
                    "Validation failed".to_string(),
                    details,
                    request_id.to_string(),
                )
            }
            Error::Payload(payload_err) => {
                payload_error_to_icon_api_v1_error(&payload_err, request_id)
            }
            Error::ValidationWithRequestId(validation_errors, _) => {
                let details = validation_errors_to_icon_api_v1_details(
                    &validation_errors,
                );
                IconApiV1Error::bad_request(
                    "Validation failed".to_string(),
                    details,
                    request_id.to_string(),
                )
            }
            Error::PayloadWithRequestId(payload_err, _) => {
                payload_error_to_icon_api_v1_error(&payload_err, request_id)
            }
        }
    }
}

impl From<Error> for ExtractorError {
    fn from(value: Error) -> Self {
        match value {
            Error::Payload(err) => err.into(),
            Error::PayloadWithRequestId(err, _) => err.into(),
            Error::Validation(err) => {
                let validation_errors = validation_errors_to_strings(&err);

                Self {
                    status_code: StatusCode::BAD_REQUEST,
                    code: "INVALID_REQUEST",
                    message: validation_errors.join("; "),
                    ..Default::default()
                }
            }
            Error::ValidationWithRequestId(err, _) => {
                let validation_errors = validation_errors_to_strings(&err);

                Self {
                    status_code: StatusCode::BAD_REQUEST,
                    code: "INVALID_REQUEST",
                    message: validation_errors.join("; "),
                    ..Default::default()
                }
            }
        }
    }
}

/// Recursively formats ValidationErrors into strings.
///
/// Args:
///   errors: The ValidationErrors struct to format.
///   parent_path: The path prefix for nested fields (e.g., "parent_struct.field").
///   output: A mutable vector to collect the formatted error strings.
fn format_validation_errors_recursive(
    errors: &validator::ValidationErrors,
    parent_path: Option<&str>,
    output: &mut Vec<String>,
) {
    tracing::info!("{:#?}", errors.errors());

    for (field, kind) in errors.errors() {
        let current_path = match parent_path {
            Some(p) => format!("{}.{}", p, field),
            None => field.to_string(),
        };

        match kind {
            validator::ValidationErrorsKind::Field(field_errors) => {
                for error in field_errors {
                    match current_path.as_str() {
                        "__all__" => {
                            let field = error.code.as_ref();
                            let message = error
                                .message
                                .clone()
                                .unwrap_or(Cow::Owned("".to_string()));

                            output.push(format!(
                                "`{}` failed validation: {}",
                                field, message
                            ));
                        }
                        _ => {
                            let error_message =
                                error.message.as_ref().unwrap_or(&error.code); // Use message or code
                            output.push(format!(
                                "`{}` failed validation: {}",
                                current_path, error_message
                            ));
                        }
                    }
                }
            }
            validator::ValidationErrorsKind::Struct(struct_errors) => {
                format_validation_errors_recursive(
                    struct_errors,
                    Some(&current_path),
                    output,
                );
            }
            validator::ValidationErrorsKind::List(list_errors) => {
                for (index, item_errors) in list_errors {
                    let item_path = format!("{}[{}]", current_path, index);
                    format_validation_errors_recursive(
                        item_errors,
                        Some(&item_path),
                        output,
                    );
                }
            }
        }
    }
}

/// Transforms ValidationErrors into a Vec of formatted error strings.
///
/// Example format: "`struct.field.name` failed validation: my validation error"
pub fn validation_errors_to_strings(
    errors: &validator::ValidationErrors,
) -> Vec<String> {
    let mut output = Vec::new();
    format_validation_errors_recursive(errors, None, &mut output);
    output
}

/// Transforms ValidationErrors into IconApiV1Detail entries with specific field information.
fn validation_errors_to_icon_api_v1_details(
    errors: &validator::ValidationErrors,
) -> Vec<IconApiV1Detail> {
    let mut details = Vec::new();
    format_validation_errors_to_details_recursive(errors, None, &mut details);

    if details.is_empty() {
        details.push(IconApiV1Detail {
            field: Some("request".to_string()),
            code: "validation_failed".to_string(),
            message: "Validation failed".to_string(),
            suggestion:
                "Check the request parameters and format of the request body"
                    .to_string(),
            documentation: "https://api/v1/api-reference".to_string(),
        });
    }

    details
}

/// Recursively formats ValidationErrors into IconApiV1Detail entries.
fn format_validation_errors_to_details_recursive(
    errors: &validator::ValidationErrors,
    parent_path: Option<&str>,
    output: &mut Vec<IconApiV1Detail>,
) {
    for (field, kind) in errors.errors() {
        let current_path = match parent_path {
            Some(p) => format!("{}.{}", p, field),
            None => field.to_string(),
        };

        match kind {
            validator::ValidationErrorsKind::Field(field_errors) => {
                for error in field_errors {
                    let (field_name, message) = match current_path.as_str() {
                        "__all__" => {
                            let field = error.code.as_ref();
                            let message = error.message.clone().unwrap_or(
                                Cow::Owned("validation failed".to_string()),
                            );
                            (field.to_string(), message.to_string())
                        }
                        _ => {
                            let error_message =
                                error.message.as_ref().unwrap_or(&error.code);
                            (current_path.clone(), error_message.to_string())
                        }
                    };

                    output.push(IconApiV1Detail {
                        field: Some(field_name),
                        code: error.code.to_string(),
                        message,
                        suggestion: "Check the field value and format"
                            .to_string(),
                        documentation: "https://api/v1/api-reference"
                            .to_string(),
                    });
                }
            }
            validator::ValidationErrorsKind::Struct(struct_errors) => {
                format_validation_errors_to_details_recursive(
                    struct_errors,
                    Some(&current_path),
                    output,
                );
            }
            validator::ValidationErrorsKind::List(list_errors) => {
                for (index, item_errors) in list_errors {
                    let item_path = format!("{}[{}]", current_path, index);
                    format_validation_errors_to_details_recursive(
                        item_errors,
                        Some(&item_path),
                        output,
                    );
                }
            }
        }
    }
}

/// Converts payload::Error to IconApiV1Error with specific field information extracted
/// from serde_path_to_error when available
fn payload_error_to_icon_api_v1_error(
    payload_err: &payload::Error,
    request_id: &Uuid,
) -> IconApiV1Error {
    match payload_err {
        payload::Error::Json(serde_err) => {
            let field_path = serde_err.path().to_string();
            let inner_message = serde_err.inner().to_string();

            let (field, message, code) = if field_path.is_empty() {
                (
                    "request".to_string(),
                    format!("Invalid JSON: {}", inner_message),
                    "invalid_json".to_string(),
                )
            } else {
                let field_name =
                    extract_nested_field_name(&field_path, &inner_message);
                let message = if inner_message.contains("missing field") {
                    // Extract the specific missing field name from the serde error message
                    if let Some(missing_field) =
                        extract_missing_field_from_message(&inner_message)
                    {
                        format!("Missing required field: {}", missing_field)
                    } else {
                        format!("Missing required field in: {}", field_name)
                    }
                } else {
                    format!(
                        "Invalid value for field '{}': {}",
                        field_name, inner_message
                    )
                };
                (field_name, message, "invalid_field".to_string())
            };

            IconApiV1Error::bad_request(
                "Invalid request payload".to_string(),
                vec![IconApiV1Detail {
                    field: Some(field),
                    code,
                    message,
                    suggestion: "Check the field value and format".to_string(),
                    documentation: "https://doc.com/v1/api-reference"
                        .to_string(),
                }],
                request_id.to_string(),
            )
        }
        payload::Error::MissingJsonContentType => IconApiV1Error::bad_request(
            "Missing content-type header".to_string(),
            vec![IconApiV1Detail {
                field: Some("Content-Type".to_string()),
                code: "missing_content_type".to_string(),
                message: "Content-Type header must be application/json"
                    .to_string(),
                suggestion: "Set Content-Type header to application/json"
                    .to_string(),
                documentation: "https://doc.com/v1/api-reference".to_string(),
            }],
            request_id.to_string(),
        ),
        payload::Error::Bytes(_) => IconApiV1Error::bad_request(
            "Request body error".to_string(),
            vec![IconApiV1Detail {
                field: Some("request".to_string()),
                code: "request_body_error".to_string(),
                message: "Unable to read request body".to_string(),
                suggestion: "Check the request body and content length"
                    .to_string(),
                documentation: "https://api/v1/api-reference".to_string(),
            }],
            request_id.to_string(),
        ),
    }
}

/// Extracts the nested field name from the path and error message
fn extract_nested_field_name(field_path: &str, inner_message: &str) -> String {
    if inner_message.contains("missing field") {
        if let Some(missing_field) =
            extract_missing_field_from_message(inner_message)
        {
            if field_path.is_empty() {
                missing_field
            } else {
                format!("{}.{}", field_path, missing_field)
            }
        } else {
            field_path.to_string()
        }
    } else {
        field_path.to_string()
    }
}

/// Extracts the specific field name from a serde "missing field" error message
/// Example: "missing field `appName`" -> Some("appName")
fn extract_missing_field_from_message(message: &str) -> Option<String> {
    if let Some(start) = message.find("missing field") {
        let after_missing = &message[start + "missing field".len()..];

        if let Some(backtick_start) = after_missing.find('`')
            && let Some(backtick_end) =
                after_missing[backtick_start + 1..].find('`')
        {
            let field_name = &after_missing
                [backtick_start + 1..backtick_start + 1 + backtick_end];
            return Some(field_name.to_string());
        }

        if let Some(quote_start) = after_missing.find('"')
            && let Some(quote_end) = after_missing[quote_start + 1..].find('"')
        {
            let field_name =
                &after_missing[quote_start + 1..quote_start + 1 + quote_end];
            return Some(field_name.to_string());
        }
    }

    None
}
