use reqwest::Client as HttpClient;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use reqwest_tracing::TracingMiddleware;
use serde::de::DeserializeOwned;
use types::{HistoryResponse, IconResponse};

use crate::error::{ApiErrorResponse, IconApiError, IconApiResult};

const API_PREFIX: &str = "/api/icons/v1";

/// Client for the Icon Lookup API. Cheap to clone (shares the underlying
/// connection pool); transient failures are retried with exponential backoff.
#[derive(Debug, Clone)]
pub struct IconClient {
    client: ClientWithMiddleware,
    base_url: String,
}

impl IconClient {
    /// Create a client pointing at `base_url` (e.g. `http://localhost:8080`).
    pub fn new(base_url: impl Into<String>) -> Self {
        let retry_policy =
            ExponentialBackoff::builder().build_with_max_retries(3);
        let client = ClientBuilder::new(HttpClient::new())
            .with(TracingMiddleware::default())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    /// Resolve the icon for a file path (DB + Redis backed).
    pub async fn file_icon(&self, path: &str) -> IconApiResult<IconResponse> {
        self.lookup("/icons/file", path).await
    }

    /// Resolve the icon for a folder path (DB + Redis backed).
    pub async fn folder_icon(&self, path: &str) -> IconApiResult<IconResponse> {
        self.lookup("/icons/folder", path).await
    }

    /// Resolve the icon for a file path from the server's in-memory maps.
    pub async fn file_icon_in_memory(
        &self,
        path: &str,
    ) -> IconApiResult<IconResponse> {
        self.lookup("/icons/memory/file", path).await
    }

    /// Resolve the icon for a folder path from the server's in-memory maps.
    pub async fn folder_icon_in_memory(
        &self,
        path: &str,
    ) -> IconApiResult<IconResponse> {
        self.lookup("/icons/memory/folder", path).await
    }

    /// Fetch the most recent DB-backed lookups.
    pub async fn history(&self) -> IconApiResult<HistoryResponse> {
        let url = format!("{}{}/icons/history", self.base_url, API_PREFIX);
        let response = self.client.get(url).send().await?;
        self.parse(response).await
    }

    #[tracing::instrument(skip(self))]
    async fn lookup(
        &self,
        endpoint: &str,
        path: &str,
    ) -> IconApiResult<IconResponse> {
        let url = format!("{}{}{}", self.base_url, API_PREFIX, endpoint);
        let response =
            self.client.get(url).query(&[("path", path)]).send().await?;
        self.parse(response).await
    }

    async fn parse<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> IconApiResult<T> {
        let status = response.status();
        if status.is_success() {
            Ok(response.json::<T>().await?)
        } else {
            let body = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiErrorResponse>(&body)
                .map(|e| e.message)
                .unwrap_or(body);
            Err(IconApiError::Api {
                status: status.as_u16(),
                message,
            })
        }
    }
}
