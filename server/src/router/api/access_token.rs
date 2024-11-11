use std::error::Error;
use std::time::Duration;

use axum::extract::rejection::{FormRejection, JsonRejection};
use axum::http::header::{ACCEPT, CONTENT_TYPE};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Form, Json};
use uuid::Uuid;

use crate::router::ui::authorize::AuthorizationState;

// 1 day
const ACCESS_TOKEN_TTL: Duration = Duration::new(60 * 60 * 24, 0);

pub(crate) struct AnyContentType<T>(pub T);

pub(crate) enum AnyContentTypeRejection {
    ContentTypeHeaderMissing,
    ContentTypeHeaderInvalid,
    ContentTypeNotSupported,
    JsonRejection(JsonRejection),
    FormRejection(FormRejection),
}

impl AnyContentTypeRejection {
    fn status_and_message(&self) -> (StatusCode, &'static str) {
        match self {
            Self::ContentTypeHeaderMissing => {
                (StatusCode::BAD_REQUEST, "no 'Content-Type' header provided")
            }
            Self::ContentTypeHeaderInvalid => (
                StatusCode::BAD_REQUEST,
                "invalid 'Content-Type' header provided",
            ),
            Self::ContentTypeNotSupported => (
                StatusCode::NOT_ACCEPTABLE,
                "provided 'Content-Type' not supported",
            ),
            Self::JsonRejection(err) => {
                let cause = err.source();
                tracing::debug!(message = "failed decoding json payload", cause = cause);
                (StatusCode::BAD_REQUEST, "unable to decode json payload")
            }
            Self::FormRejection(err) => {
                let cause = err.source();
                tracing::debug!(message = "failed decoding form payload", cause = cause);
                (StatusCode::BAD_REQUEST, "unable to decode form payload")
            }
        }
    }
}

impl IntoResponse for AnyContentTypeRejection {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = self.status_and_message();
        super::error::Error::new(status, message).into_response()
    }
}

#[axum::async_trait]
impl<T, S> axum::extract::FromRequest<S> for AnyContentType<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AnyContentTypeRejection;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .ok_or(AnyContentTypeRejection::ContentTypeHeaderMissing)?;
        let content_type = content_type
            .to_str()
            .map_err(|_| AnyContentTypeRejection::ContentTypeHeaderInvalid)?;
        if content_type.starts_with("application/json") {
            Json::from_request(req, state)
                .await
                .map(|Json(inner)| AnyContentType(inner))
                .map_err(AnyContentTypeRejection::JsonRejection)
        } else if content_type.starts_with("application/x-www-form-urlencoded") {
            Form::from_request(req, state)
                .await
                .map(|Form(inner)| AnyContentType(inner))
                .map_err(AnyContentTypeRejection::FormRejection)
        } else {
            Err(AnyContentTypeRejection::ContentTypeNotSupported)
        }
    }
}

pub enum ResponseError {
    CodeNotFound,
    ApplicationNotFound,
    InvalidRedirectUri,
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::response::Response {
        StatusCode::NOT_IMPLEMENTED.into_response()
    }
}

#[derive(serde::Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
pub(crate) struct RequestPayload {
    code: String,
    grant_type: String,
    code_verifier: String,
    redirect_uri: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum AcceptHeader {
    Json,
    #[default]
    Form,
}

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AcceptHeader
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match parts
            .headers
            .get(ACCEPT)
            .and_then(|value| value.to_str().ok())
        {
            Some("application/json") => Ok(AcceptHeader::Json),
            Some("application/x-www-form-urlencoded") | None => Ok(AcceptHeader::Form),
            Some(other) => {
                tracing::warn!("received a request for accept header of type {other}");
                Err((
                    StatusCode::NOT_ACCEPTABLE,
                    "`Accept` header is requesting an incompatible type",
                ))
            }
        }
    }
}

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "snake_case")]
pub(crate) enum TokenType {
    Bearer,
}

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub(crate) struct ResponsePayload {
    #[serde(skip)]
    accept: AcceptHeader,
    access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    token_type: TokenType,
}

impl IntoResponse for ResponsePayload {
    fn into_response(self) -> axum::response::Response {
        match self.accept {
            AcceptHeader::Json => Json(self).into_response(),
            AcceptHeader::Form => Form(self).into_response(),
        }
    }
}

fn generate_token() -> String {
    use rand::distributions::{Alphanumeric, Distribution};
    use rand::thread_rng;

    let mut rng = thread_rng();
    Alphanumeric
        .sample_iter(&mut rng)
        .take(42)
        .map(char::from)
        .collect()
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct SessionState {
    pub client_id: String,
    pub user: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

pub(super) async fn handle(
    Extension(cache): Extension<crate::service::cache::Client>,
    Extension(dataset): Extension<crate::service::dataset::Client>,
    accept: AcceptHeader,
    AnyContentType(payload): AnyContentType<RequestPayload>,
) -> Result<ResponsePayload, ResponseError> {
    let state: AuthorizationState = cache
        .remove(&payload.code)
        .await
        .ok_or(ResponseError::CodeNotFound)?;

    let app = dataset
        .find(&state.client_id)
        .ok_or(ResponseError::ApplicationNotFound)?;
    if !app.check_redirect_uri(payload.redirect_uri.as_str()) {
        return Err(ResponseError::InvalidRedirectUri);
    }

    let access_token = generate_token();
    cache
        .insert(
            access_token.clone(),
            &SessionState {
                client_id: state.client_id,
                user: state.user,
                scope: state.scope.clone(),
            },
            ACCESS_TOKEN_TTL,
        )
        .await;

    Ok(ResponsePayload {
        accept,
        access_token,
        scope: state.scope,
        token_type: TokenType::Bearer,
    })
}

#[cfg(test)]
mod integration_tests {
    use std::time::Duration;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt; // for `collect`

    use crate::{
        router::ui::authorize::AuthorizationState,
        service::dataset::{ALICE_ID, APP_ID, REDIRECT_URI},
    };

    const SHORT_TTL: Duration = Duration::new(5, 0);

    #[tokio::test]
    async fn should_create_access_token_without_defined_type() {
        crate::enable_tracing();

        let app = crate::app::Application::test();
        app.cache()
            .insert(
                "aaaaaaaaaaaaaaaaaaa".into(),
                &AuthorizationState {
                    state: "state".into(),
                    scope: None,
                    client_id: APP_ID.into(),
                    user: ALICE_ID,
                },
                SHORT_TTL,
            )
            .await;

        let req = Request::builder()
            .uri("/api/access-token")
            .header("Content-Type", "application/json")
            .method("POST")
            .body(Body::from(
                serde_json::to_vec(&super::RequestPayload {
                    code: "aaaaaaaaaaaaaaaaaaa".into(),
                    code_verifier: "whatever".into(),
                    grant_type: "".into(),
                    redirect_uri: REDIRECT_URI.into(),
                })
                .unwrap(),
            ))
            .unwrap();
        let res = app.handle(req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let ctype = res
            .headers()
            .get("Content-Type")
            .and_then(|h| h.to_str().ok())
            .unwrap();
        assert_eq!(ctype, "application/x-www-form-urlencoded");
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body: super::ResponsePayload = serde_urlencoded::from_bytes(&body).unwrap();
        assert!(body.scope.is_none())
    }

    #[tokio::test]
    async fn should_create_access_token_with_json_type() {
        crate::enable_tracing();

        let app = crate::app::Application::test();
        app.cache()
            .insert(
                "aaaaaaaaaaaaaaaaaaa".into(),
                &AuthorizationState {
                    state: "state".into(),
                    scope: None,
                    client_id: APP_ID.into(),
                    user: ALICE_ID,
                },
                SHORT_TTL,
            )
            .await;

        let req = Request::builder()
            .uri("/api/access-token")
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .method("POST")
            .body(Body::from(
                serde_json::to_vec(&super::RequestPayload {
                    code: "aaaaaaaaaaaaaaaaaaa".into(),
                    code_verifier: "whatever".into(),
                    grant_type: "".into(),
                    redirect_uri: REDIRECT_URI.into(),
                })
                .unwrap(),
            ))
            .unwrap();
        let res = app.handle(req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let ctype = res
            .headers()
            .get("Content-Type")
            .and_then(|h| h.to_str().ok())
            .unwrap();
        assert_eq!(ctype, "application/json");
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body: super::ResponsePayload = serde_json::from_slice(&body).unwrap();
        assert!(body.scope.is_none())
    }

    #[tokio::test]
    async fn should_create_access_token_with_form_type() {
        crate::enable_tracing();

        let app = crate::app::Application::test();
        app.cache()
            .insert(
                "aaaaaaaaaaaaaaaaaaa".into(),
                &AuthorizationState {
                    state: "state".into(),
                    scope: None,
                    client_id: APP_ID.into(),
                    user: ALICE_ID,
                },
                SHORT_TTL,
            )
            .await;

        let req = Request::builder()
            .uri("/api/access-token")
            .header("Accept", "application/x-www-form-urlencoded")
            .header("Content-Type", "application/json")
            .method("POST")
            .body(Body::from(
                serde_json::to_vec(&super::RequestPayload {
                    code: "aaaaaaaaaaaaaaaaaaa".into(),
                    code_verifier: "whatever".into(),
                    grant_type: "".into(),
                    redirect_uri: REDIRECT_URI.into(),
                })
                .unwrap(),
            ))
            .unwrap();
        let res = app.handle(req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let ctype = res
            .headers()
            .get("Content-Type")
            .and_then(|h| h.to_str().ok())
            .unwrap();
        assert_eq!(ctype, "application/x-www-form-urlencoded");
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body: super::ResponsePayload = serde_urlencoded::from_bytes(&body).unwrap();
        assert!(body.scope.is_none())
    }
}
