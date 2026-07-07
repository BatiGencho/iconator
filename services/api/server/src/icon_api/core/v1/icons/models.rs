use serde::Deserialize;
use utoipa::IntoParams;

// The response DTOs are the shared API contract, defined once in `types` and
// consumed by both this server and the `sdk` client.
pub use types::{IconResponse, IconSource};

/// Query string for the icon lookup endpoints, e.g. `?path=./src/main.rs`.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct IconQuery {
    /// The file or folder path to resolve an icon for.
    #[param(example = "./src/main.rs")]
    pub path: String,
}
