use axum::http::StatusCode;

pub async fn handler() -> StatusCode {
    tracing::trace!("status requested");
    StatusCode::NO_CONTENT
}
