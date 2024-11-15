use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::{AppendHeaders, IntoResponse};
use axum::routing::get;

async fn handle_style_css() -> impl IntoResponse {
    (
        AppendHeaders([
            (CACHE_CONTROL, "public, max-age=31536000, immutable"),
            (CONTENT_TYPE, "text/css"),
        ]),
        tekitoi_ui::asset::STYLE_CSS_CONTENT,
    )
}

pub(super) fn router() -> axum::Router {
    axum::Router::new().route(tekitoi_ui::asset::STYLE_CSS_PATH, get(handle_style_css))
}
