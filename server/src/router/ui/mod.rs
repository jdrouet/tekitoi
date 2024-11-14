use axum::routing::get;

pub(super) mod authorize;
mod error;
mod helper;
mod login;

pub(super) fn router() -> axum::Router {
    axum::Router::new()
        .route("/authorize", get(authorize::handle))
        .route("/authorize/profiles/login", get(login::profiles::handle))
}
