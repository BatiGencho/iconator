//! # Icon API Server
//!
use crate::metrics::ServerMetrics;
use crate::shutdown::ShutdownCoordinator;
use std::sync::Arc;
use telemetry::metrics::Telemetry;
// Private API modules - internal implementation details
mod icon_api;
pub mod shutdown;

// OpenAPI documentation module
pub mod openapi;

// Public modules - shared utilities and middleware
// These provide common functionality that can be used across the application
pub mod health;
pub mod metrics;
pub mod shared;

// Public API surface - only expose route registration functions
// This provides a clean API boundary where external code can only access
// the route registration functions without depending on internal module structure

pub use icon_api::core::v1::get_routes as get_icon_api_v1_routes;

/// Returns the OpenAPI documentation routes for Icon v1 API
/// Includes Swagger UI and OpenAPI JSON spec with OpenAPI 3.0 compatibility fixes
pub fn get_openapi_routes() -> axum::Router {
    use axum::Json;
    use axum::routing::get;
    use utoipa_swagger_ui::SwaggerUi;

    // Custom handler that serves OpenAPI 3.0 compatible JSON (for Mintlify)
    // Converts type: ["array", "null"] -> type: "array", nullable: true
    async fn openapi_3_0_handler() -> Json<serde_json::Value> {
        Json(openapi::IconApiV1Doc::openapi_json())
    }

    axum::Router::new()
        .without_v07_checks()
        // OpenAPI 3.0 format with nullable array fixes
        // Keep as /api-docs/openapi.json for backward compatibility
        .route("/api-docs/openapi.json", get(openapi_3_0_handler))
        // SwaggerUI: Creates /api-docs/openapi-3.1.json serving native OpenAPI 3.1 spec
        // SwaggerUI handles type: ["array", "null"] correctly, so no conversion needed
        .merge(SwaggerUi::new("/swagger-ui").url(
            "/api-docs/openapi-3.1.json",
            openapi::IconApiV1Doc::openapi(),
        ))
}

#[derive(Clone)]
pub struct AppState {
    pub telemetry: Arc<Telemetry<ServerMetrics>>,
    pub pool: postgres::connection::Pool,
    pub read_only_pool: postgres::connection::Pool,
    pub cache_pool: redis::connection::Pool,
    pub config: Arc<Config>,
    pub shutdown: Arc<ShutdownCoordinator>,
}

impl AppState {}

impl axum::extract::FromRef<AppState> for postgres::connection::Pool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

#[derive(serde::Deserialize)]
pub struct Config {
    // Service port
    pub api_service_port: String,

    // Loggers
    pub rust_log: String,
    #[serde(default)]
    pub log_format: String,

    // Postgres connection URLs. `database_ro_url` defaults to `database_url`
    // when unset - single-node/local setups use one endpoint for both.
    pub database_url: String,
    #[serde(default)]
    pub database_ro_url: Option<String>,

    // Redis configs
    pub redis_url: String,
}

impl Config {
    pub fn load() -> Result<Self, envy::Error> {
        // Load .env file if present (useful when running outside docker-compose)
        match dotenv::dotenv() {
            Ok(path) => eprintln!("Loaded .env from: {}", path.display()),
            Err(e) => eprintln!("dotenv warning: {e}"),
        }

        envy::from_env::<Config>()
    }

    /// Read-only connection URL, falling back to the read-write URL when unset.
    pub fn database_ro_url(&self) -> String {
        self.database_ro_url
            .clone()
            .unwrap_or_else(|| self.database_url.clone())
    }
}
