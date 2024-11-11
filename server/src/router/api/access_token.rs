use axum::extract::rejection::{BytesRejection, JsonRejection};
use axum::http::header::{ACCEPT, CONTENT_TYPE};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use uuid::Uuid;

use crate::router::ui::authorize::AuthorizationState;

pub(crate) struct AnyContentType<T>(pub T);

pub(crate) enum AnyContentTypeRejection {
    ContentTypeHeaderMissing,
    ContentTypeHeaderInvalid,
    JsonRejection(JsonRejection),
    BytesRejection(BytesRejection),
    BytesInvalid(serde_urlencoded::de::Error),
    ContentTypeNotSupported,
}

impl IntoResponse for AnyContentTypeRejection {
    fn into_response(self) -> axum::response::Response {
        StatusCode::NOT_ACCEPTABLE.into_response()
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
        } else {
            axum::body::Bytes::from_request(req, state)
                .await
                .map_err(AnyContentTypeRejection::BytesRejection)
                .and_then(|bytes| {
                    serde_urlencoded::from_bytes::<'_, T>(&bytes)
                        .map(AnyContentType)
                        .map_err(AnyContentTypeRejection::BytesInvalid)
                })
        }
    }
}

pub enum ResponseError {
    CodeNotFound,
    InvalidClientId,
    ApplicationNotFound,
    InvalidRedirectUri,
    InvalidClientSecret,
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::response::Response {
        StatusCode::NOT_IMPLEMENTED.into_response()
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct RequestPayload {
    client_id: String,
    client_secret: String,
    code: String,
    redirect_uri: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum AcceptHeader {
    Json,
    Default,
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
            Some(_other) => Err((
                StatusCode::NOT_ACCEPTABLE,
                "`Accept` header is requesting an incompatible type",
            )),
            None => Ok(AcceptHeader::Default),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TokenType {
    Bearer,
}

#[derive(serde::Serialize)]
pub(crate) struct ResponsePayload {
    #[serde(skip)]
    accept: AcceptHeader,
    access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    token_stype: TokenType,
}

impl IntoResponse for ResponsePayload {
    fn into_response(self) -> axum::response::Response {
        match self.accept {
            AcceptHeader::Json => Json(self).into_response(),
            AcceptHeader::Default => match serde_urlencoded::to_string(&self) {
                Ok(value) => value.into_bytes().into_response(),
                Err(_err) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            },
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

impl SessionState {
    pub fn new(client_id: String, user: Uuid, scope: Option<String>) -> Self {
        Self {
            client_id,
            user,
            scope,
        }
    }

    pub fn serialize(&self) -> String {
        serde_urlencoded::to_string(self).unwrap()
    }

    pub fn deserialize(input: &str) -> Self {
        serde_urlencoded::from_str(input).unwrap()
    }
}

pub(super) async fn handle(
    Extension(cache): Extension<crate::service::cache::Client>,
    Extension(dataset): Extension<crate::service::dataset::Client>,
    accept: AcceptHeader,
    AnyContentType(payload): AnyContentType<RequestPayload>,
) -> Result<ResponsePayload, ResponseError> {
    let state = cache
        .remove(&payload.code)
        .await
        .ok_or(ResponseError::CodeNotFound)?;
    let state = AuthorizationState::deserialize(&state);
    if payload.client_id != state.client_id {
        return Err(ResponseError::InvalidClientId);
    }

    let app = dataset
        .find(&payload.client_id)
        .ok_or(ResponseError::ApplicationNotFound)?;
    if !app.check_redirect_uri(payload.redirect_uri.as_str()) {
        return Err(ResponseError::InvalidRedirectUri);
    }
    if !app.check_client_secret(payload.client_secret.as_str()) {
        return Err(ResponseError::InvalidClientSecret);
    }

    let access_token = generate_token();
    cache
        .insert(
            access_token.clone(),
            SessionState::new(state.client_id, state.user, state.scope.clone()).serialize(),
        )
        .await;

    Ok(ResponsePayload {
        accept,
        access_token,
        scope: state.scope,
        token_stype: TokenType::Bearer,
    })
}
