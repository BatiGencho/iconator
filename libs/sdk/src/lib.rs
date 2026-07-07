//! Async client SDK for the Icon Lookup API.
//!
//! ```no_run
//! # async fn demo() -> sdk::IconApiResult<()> {
//! let client = sdk::IconClient::new("http://localhost:8080");
//! let icon = client.file_icon("./src/main.rs").await?;
//! println!("icon id: {:?}", icon.icon_id);
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;

pub use client::IconClient;
pub use error::{IconApiError, IconApiResult};

// Re-export the shared API types so callers don't need a direct `types` dependency.
pub use types::{HistoryResponse, IconQueryEntry, IconResponse, IconSource};
