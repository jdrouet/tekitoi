mod api;

pub(crate) fn create() -> axum::Router {
    axum::Router::new().nest("/api", api::router())
}
