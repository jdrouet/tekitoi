use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::{AppendHeaders, IntoResponse};
use axum::routing::get;

pub(crate) const STYLE_PATH: &str = concat!("/assets/style-", env!("CARGO_PKG_VERSION"), ".css");

async fn handle_style_css() -> impl IntoResponse {
    let payload = include_bytes!("style.css");
    (
        AppendHeaders([
            (CACHE_CONTROL, "public, max-age=31536000, immutable"),
            (CONTENT_TYPE, "text/css"),
        ]),
        payload,
    )
}

pub(super) fn router() -> axum::Router {
    axum::Router::new().route(STYLE_PATH, get(handle_style_css))
}
