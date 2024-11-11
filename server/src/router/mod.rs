mod api;
mod ui;

pub(crate) fn create() -> axum::Router {
    axum::Router::new()
        .nest("/api", api::router())
        .merge(ui::router())
}
