use axum::Router;

pub(crate) mod icons;

pub fn get_routes(state: crate::AppState) -> Router {
    Router::new().nest("/icons", icons::get_routes(state))
}
