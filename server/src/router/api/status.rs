use axum::{http::StatusCode, Extension};

pub(crate) async fn handle(
    Extension(database): Extension<crate::service::database::Pool>,
) -> StatusCode {
    match database.ping().await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(err) => {
            tracing::error!(message = "unable to ping database", error = %err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
