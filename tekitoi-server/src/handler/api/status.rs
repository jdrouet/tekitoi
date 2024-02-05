use axum::http::StatusCode;

pub(crate) async fn handler() -> StatusCode {
    StatusCode::NO_CONTENT
}
