mod api;
mod asset;
mod ui;

pub(crate) fn create() -> axum::Router {
    axum::Router::new()
        .nest("/api", api::router())
        .merge(asset::router())
        .merge(ui::router())
}
