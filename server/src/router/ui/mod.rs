use axum::routing::get;

pub(super) mod authorize;
mod helper;

pub(super) fn router() -> axum::Router {
    axum::Router::new().route("/authorize", get(authorize::handle))
}
