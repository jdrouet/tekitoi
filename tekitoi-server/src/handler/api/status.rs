use axum::http::StatusCode;

// #[get("/api/status")]
pub async fn handler() -> StatusCode {
    StatusCode::NO_CONTENT
}
