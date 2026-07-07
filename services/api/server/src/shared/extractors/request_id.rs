use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

/// Extract request ID from the `x-request-id` header set by Envoy
///
/// If the header is missing or invalid, generates a new UUID for this request
#[derive(Debug, Clone, Copy)]
pub struct RequestId(pub Uuid);

impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        let request_id = parts
            .headers
            .get("x-request-id")
            .and_then(|header| header.to_str().ok())
            .and_then(|header_str| Uuid::parse_str(header_str).ok())
            .unwrap_or_else(Uuid::new_v4);

        Ok(Self(request_id))
    }
}

impl std::ops::Deref for RequestId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
