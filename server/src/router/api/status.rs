use axum::http::StatusCode;

pub(crate) async fn handle() -> StatusCode {
    StatusCode::NO_CONTENT
}
