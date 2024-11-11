use axum::routing::{get, post};

mod access_token;
mod status;
mod user_info;

pub(super) fn router() -> axum::Router {
    axum::Router::new()
        .route("/access-token", post(access_token::handle))
        .route("/status", get(status::handle))
        .route("/user-info", get(user_info::handle))
}
