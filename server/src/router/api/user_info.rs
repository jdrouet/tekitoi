use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};

use super::access_token::SessionState;
use super::prelude::AuthorizationToken;
use crate::entity::user::Entity as UserEntity;

#[derive(Debug)]
pub(crate) enum ErrorResponse {
    UnknownApplication,
    TokenNotFound,
    UnknownUser,
}

impl ErrorResponse {
    fn status_and_message(&self) -> (StatusCode, &'static str) {
        match self {
            Self::UnknownApplication => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "unknown related application",
            ),
            Self::TokenNotFound => (StatusCode::UNAUTHORIZED, "invalid token"),
            Self::UnknownUser => (StatusCode::INTERNAL_SERVER_ERROR, "unknown relatd user"),
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let (code, message) = self.status_and_message();
        super::error::Error::new(code, message).into_response()
    }
}

#[axum::debug_handler]
pub(super) async fn handle(
    Extension(cache): Extension<crate::service::cache::Client>,
    Extension(dataset): Extension<crate::service::dataset::Client>,
    AuthorizationToken(token): AuthorizationToken,
) -> Result<Json<UserEntity>, ErrorResponse> {
    let session: SessionState = cache
        .get(token.token())
        .await
        .ok_or(ErrorResponse::TokenNotFound)?;
    let app = dataset
        .find(&session.client_id)
        .ok_or(ErrorResponse::UnknownApplication)?;
    let user = app.user(session.user).ok_or(ErrorResponse::UnknownUser)?;

    Ok(Json(user.clone()))
}

#[cfg(test)]
mod integration_tests {
    use std::time::Duration;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use uuid::Uuid;

    use crate::{router::api::access_token::SessionState, service::dataset::ALICE_ID};

    const LOCAL_TTL: Duration = Duration::new(10, 0);

    #[tokio::test]
    async fn should_return_user() {
        let app = crate::app::Application::test();
        app.cache()
            .insert(
                "aaaaaaaaaaaaaaaaaaa".into(),
                &SessionState::new("client-id".into(), ALICE_ID, None),
                LOCAL_TTL,
            )
            .await;

        let req = Request::builder()
            .uri("/api/user-info")
            .header("Authorization", "Bearer aaaaaaaaaaaaaaaaaaa")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let res = app.handle(req).await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn missing_access_token_should_fail() {
        let app = crate::app::Application::test();

        let req = Request::builder()
            .uri("/api/user-info")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let res = app.handle(req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn unknown_app_should_fail() {
        let app = crate::app::Application::test();
        app.cache()
            .insert(
                "aaaaaaaaaaaaaaaaaaa".into(),
                &SessionState::new("unknown".into(), ALICE_ID, None),
                LOCAL_TTL,
            )
            .await;

        let req = Request::builder()
            .uri("/api/user-info")
            .method("GET")
            .header("Authorization", "Bearer aaaaaaaaaaaaaaaaaaa")
            .body(Body::empty())
            .unwrap();
        let res = app.handle(req).await;
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn unknown_user_should_fail() {
        let app = crate::app::Application::test();
        app.cache()
            .insert(
                "aaaaaaaaaaaaaaaaaaa".into(),
                &SessionState::new("client-id".into(), Uuid::new_v4(), None),
                LOCAL_TTL,
            )
            .await;

        let req = Request::builder()
            .uri("/api/user-info")
            .method("GET")
            .header("Authorization", "Bearer aaaaaaaaaaaaaaaaaaa")
            .body(Body::empty())
            .unwrap();
        let res = app.handle(req).await;
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
