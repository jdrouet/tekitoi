use axum::routing::get;

mod status;

pub(super) fn router() -> axum::Router {
    axum::Router::new().route("/status", get(status::handle))
}
