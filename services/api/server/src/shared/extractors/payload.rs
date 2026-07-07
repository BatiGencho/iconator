use axum::extract::rejection::BytesRejection;
use axum::extract::{FromRequest, Request};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde_json::error::Category;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Payload<T>(pub T);

impl<T, S> FromRequest<S> for Payload<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(
        req: Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        if json_content_type(req.headers()) {
            let bytes = Bytes::from_request(req, state).await?;
            let deserializer =
                &mut serde_json::Deserializer::from_slice(&bytes);
            let value: T = serde_path_to_error::deserialize(deserializer)?;

            Ok(Payload(value))
        } else {
            Err(Error::MissingJsonContentType)
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Bytes(#[from] BytesRejection),

    #[error(transparent)]
    Json(#[from] serde_path_to_error::Error<serde_json::Error>),

    #[error("missing content-type header")]
    MissingJsonContentType,
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> Response {
        let app_error: crate::shared::extractors::error::Error = self.into();
        app_error.into_response()
    }
}

impl From<Error> for crate::shared::extractors::error::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::Json(err) => match err.inner().classify() {
                Category::Data => {
                    let path = err.path().to_string();
                    let inner_err = err.inner().to_string();

                    let message = match inner_err.contains("missing field") {
                        true => inner_err,
                        false => {
                            format!("`{}` is invalid json: {}", path, inner_err)
                        }
                    };

                    Self {
                        status_code: StatusCode::BAD_REQUEST,
                        code: "INVALID_REQUEST",
                        message,
                        ..Default::default()
                    }
                }
                _ => Self {
                    status_code: StatusCode::BAD_REQUEST,
                    code: "INVALID_REQUEST",
                    message: format!("{:#?}", err),
                    ..Default::default()
                },
            },
            _ => Self {
                status_code: StatusCode::BAD_REQUEST,
                code: "INVALID_REQUEST",
                message: format!("{}", value),
                ..Default::default()
            },
        }
    }
}

fn json_content_type(headers: &HeaderMap) -> bool {
    let content_type =
        if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
            content_type
        } else {
            return false;
        };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        return false;
    };

    let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
        mime
    } else {
        return false;
    };

    mime.type_() == "application"
        && (mime.subtype() == "json"
            || mime.suffix().is_some_and(|name| name == "json"))
}
