use axum::Router;
use axum::routing::get;

pub mod db;
mod errors;
pub mod history;
pub mod memory;
pub mod models;
mod resolve;

/// Icon lookup routes, mounted under `/icons`.
///
///   GET /icons/file          ?path=...  DB + Redis
///   GET /icons/folder        ?path=...  DB + Redis
///   GET /icons/memory/file   ?path=...  in-memory (iconator fst)
///   GET /icons/memory/folder ?path=...  in-memory (iconator fst)
///   GET /icons/history                  last 10 DB-backed lookups
pub fn get_routes(state: crate::AppState) -> Router {
    Router::new()
        .route("/file", get(db::file))
        .route("/folder", get(db::folder))
        .route("/memory/file", get(memory::file))
        .route("/memory/folder", get(memory::folder))
        .route("/history", get(history::handler))
        .with_state(state)
}
