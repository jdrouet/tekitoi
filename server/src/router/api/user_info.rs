use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};

use super::prelude::AuthorizationToken;
use crate::entity::user::Entity as UserEntity;

#[derive(Debug)]
pub(crate) enum ErrorResponse {
    TokenNotFound,
    UnknownUser,
    Database,
}

impl From<sqlx::Error> for ErrorResponse {
    fn from(value: sqlx::Error) -> Self {
        tracing::error!(message = "database interaction failed", error = %value);
        Self::Database
    }
}

impl ErrorResponse {
    fn status_and_message(&self) -> (StatusCode, &'static str) {
        match self {
            Self::TokenNotFound => (StatusCode::UNAUTHORIZED, "invalid token"),
            Self::UnknownUser => (StatusCode::INTERNAL_SERVER_ERROR, "unknown relatd user"),
            Self::Database => (StatusCode::INTERNAL_SERVER_ERROR, "something went wrong..."),
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
    Extension(database): Extension<crate::service::database::Pool>,
    AuthorizationToken(token): AuthorizationToken,
) -> Result<Json<UserEntity>, ErrorResponse> {
    let mut tx = database.as_ref().begin().await?;
    let session = crate::entity::session::FindByAccessToken::new(token.token())
        .execute(&mut *tx)
        .await?;
    let session = session.ok_or(ErrorResponse::TokenNotFound)?;

    let user = crate::entity::user::FindById::new(session.user_id, session.client_id)
        .execute(&mut *tx)
        .await?;

    let user = user.ok_or(ErrorResponse::UnknownUser)?;

    Ok(Json(user))
}

#[cfg(test)]
mod integration_tests {
    use std::time::Duration;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use uuid::Uuid;

    use crate::service::dataset::{ALICE_ID, APP_ID};

    const LOCAL_TTL: Duration = Duration::new(10, 0);

    #[tokio::test]
    async fn should_return_user() {
        crate::enable_tracing();
        let app = crate::app::Application::test().await;
        crate::entity::session::Create {
            access_token: "aaaaaaaaaaaaaaaaaaa".into(),
            client_id: APP_ID,
            user_id: ALICE_ID,
            scope: None,
            time_to_live: LOCAL_TTL,
        }
        .execute(app.database())
        .await
        .unwrap();

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
        crate::enable_tracing();
        let app = crate::app::Application::test().await;

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
        crate::enable_tracing();

        let app = crate::app::Application::test().await;
        crate::entity::session::Create {
            access_token: "aaaaaaaaaaaaaaaaaaa".into(),
            client_id: Uuid::new_v4(),
            user_id: ALICE_ID,
            scope: None,
            time_to_live: LOCAL_TTL,
        }
        .execute(app.database())
        .await
        .unwrap();

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
        crate::enable_tracing();
        let app = crate::app::Application::test().await;
        crate::entity::session::Create {
            access_token: "aaaaaaaaaaaaaaaaaaa".into(),
            client_id: APP_ID,
            user_id: Uuid::new_v4(),
            scope: None,
            time_to_live: LOCAL_TTL,
        }
        .execute(app.database())
        .await
        .unwrap();

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
