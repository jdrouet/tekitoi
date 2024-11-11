use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use super::access_token::SessionState;
use crate::entity::user::Entity as UserEntity;

#[derive(Debug)]
pub(crate) enum ErrorResponse {
    ApplicationNotFound,
    TokenNotFound,
    UserNotFound,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        // TODO
        StatusCode::UNAUTHORIZED.into_response()
    }
}

#[axum::debug_handler]
pub(super) async fn handle(
    Extension(cache): Extension<crate::service::cache::Client>,
    Extension(dataset): Extension<crate::service::dataset::Client>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<UserEntity>, ErrorResponse> {
    let session = cache
        .get(token.token())
        .await
        .ok_or(ErrorResponse::TokenNotFound)?;
    let session = SessionState::deserialize(&session);
    let app = dataset
        .find(&session.client_id)
        .ok_or(ErrorResponse::ApplicationNotFound)?;
    let user = app.user(session.user).ok_or(ErrorResponse::UserNotFound)?;

    Ok(Json(user.clone()))
}
