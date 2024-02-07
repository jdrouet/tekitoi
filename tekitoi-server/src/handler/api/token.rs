use super::error::ApiError;
use crate::model::provider::GetProviderById;
use crate::model::provider_authorization_request::GetProviderAuthorizationRequestById;
use crate::model::redirected::FindRedirectedRequestByCode;
use crate::model::token::CreateAccessToken;
use crate::service::database::DatabasePool;
use crate::service::BaseUrl;
use axum::body::Body;
use axum::extract::rejection::{FormRejection, JsonRejection};
use axum::extract::{FromRequest, Request};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, Response};
use axum::response::IntoResponse;
use axum::{Extension, Form, Json};
use chrono::Duration;
use oauth2::basic::BasicTokenType;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AccessToken, AuthorizationCode, EmptyExtraTokenFields, PkceCodeVerifier, StandardTokenResponse,
    TokenResponse,
};
use url::Url;

fn is_json_content(headers: &HeaderMap) -> bool {
    let Some(content_type) = headers.get(CONTENT_TYPE) else {
        return false;
    };

    let Ok(content_type) = content_type.to_str() else {
        return false;
    };

    content_type.starts_with("application/json")
}

#[derive(Debug)]
pub(crate) enum AccessTokenRequestPayloadParseError {
    Json(JsonRejection),
    Form(FormRejection),
}

impl IntoResponse for AccessTokenRequestPayloadParseError {
    fn into_response(self) -> Response<Body> {
        match self {
            Self::Form(inner) => inner.into_response(),
            Self::Json(inner) => inner.into_response(),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub(crate) struct AccessTokenRequestPayload {
    pub code: String,
    pub code_verifier: String,
    pub grant_type: String,
    pub redirect_uri: Url,
}

#[axum::async_trait]
impl<S> FromRequest<S> for AccessTokenRequestPayload
where
    S: Send + Sized + Sync,
{
    type Rejection = AccessTokenRequestPayloadParseError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if is_json_content(req.headers()) {
            Json::<AccessTokenRequestPayload>::from_request(req, state)
                .await
                .map(|Json(inner)| inner)
                .map_err(AccessTokenRequestPayloadParseError::Json)
        } else {
            Form::<AccessTokenRequestPayload>::from_request(req, state)
                .await
                .map(|Form(inner)| inner)
                .map_err(AccessTokenRequestPayloadParseError::Form)
        }
    }
}

pub(crate) async fn handler(
    Extension(base_url): Extension<BaseUrl>,
    Extension(pool): Extension<DatabasePool>,
    payload: AccessTokenRequestPayload,
) -> Result<Json<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>>, ApiError> {
    let mut tx = pool.begin().await?;

    let redirected_request = FindRedirectedRequestByCode::new(&payload.code)
        .execute(&mut tx)
        .await?;

    let Some(redirected_request) = redirected_request else {
        tracing::debug!("couldn't find redirected request by code={}", payload.code);
        return Err(ApiError::bad_request(
            "unable to find authorization request",
        ));
    };
    let provider_authorization_request = GetProviderAuthorizationRequestById::new(
        redirected_request.provider_authorization_request_id,
    )
    .execute(&mut tx)
    .await?;

    let provider = GetProviderById::new(provider_authorization_request.provider_id)
        .execute(&mut tx)
        .await?;
    //
    let oauth_client = provider.oauth_client(base_url.as_ref());
    // Now you can trade it for an access token.
    let token_result = oauth_client
        .exchange_code(AuthorizationCode::new(redirected_request.code))
        // Set the PKCE code verifier.
        .set_pkce_verifier(PkceCodeVerifier::new(
            provider_authorization_request.pkce_verifier,
        ))
        .request_async(async_http_client)
        .await
        .map_err(|err| {
            match err {
                oauth2::RequestTokenError::Parse(_, ref data) => {
                    tracing::debug!("unable to parse token: {:?}", std::str::from_utf8(data));
                }
                _ => {
                    tracing::debug!("unable to fetch token: {:?}", err);
                }
            };
            ApiError::internal_server(err.to_string())
        })?;

    let duration = token_result
        .expires_in()
        .map(|dur| Duration::seconds(dur.as_secs() as i64));

    let token_id = CreateAccessToken::new(
        redirected_request.id,
        token_result.access_token().secret(),
        duration,
    )
    .execute(&mut tx)
    .await?;

    // TODO find a better access token than a UUID
    let mut token_response = StandardTokenResponse::<EmptyExtraTokenFields, BasicTokenType>::new(
        AccessToken::new(token_id.to_string()),
        BasicTokenType::Bearer,
        EmptyExtraTokenFields {},
    );
    token_response.set_expires_in(token_result.expires_in().as_ref());

    tx.commit().await?;
    //
    Ok(Json(token_response))
}

#[cfg(test)]
mod tests {
    use crate::model::application::FindApplicationByClientId;
    use crate::model::application_authorization_request::CreateApplicationAuthorizationRequest;
    use crate::model::provider::ListProviderByApplicationId;
    use crate::model::provider_authorization_request::CreateProviderAuthorizationRequest;
    use crate::model::redirected::CreateRedirectedRequest;
    use crate::service::client::oauth::OauthProviderConfig;
    use crate::service::client::{
        ApplicationCollectionConfig, ApplicationConfig, ProviderCollectionConfig, ProviderConfig,
    };
    use crate::{settings::Settings, Server};
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::StatusCode;
    use http_body_util::BodyExt;
    use oauth2::basic::BasicTokenType;
    use oauth2::{CsrfToken, EmptyExtraTokenFields, StandardTokenResponse, TokenResponse};
    use std::path::PathBuf;
    use tower::util::ServiceExt;
    use url::Url;

    fn settings() -> Settings {
        Settings::build(Some(PathBuf::from("./tests/simple.toml")))
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn success() {
        crate::init_logger();

        let mut mock = mockito::Server::new_async().await;

        let mut settings = settings();
        settings.applications = ApplicationCollectionConfig(
            [(
                "main".to_string(),
                ApplicationConfig {
                    label: None,
                    client_id: "main-client-id".to_string(),
                    client_secrets: vec!["main-client-secret".to_string()],
                    redirect_uri: Url::parse("http://localhost/api/redirect").unwrap(),

                    providers: ProviderCollectionConfig(
                        [(
                            "local".to_string(),
                            ProviderConfig {
                                label: None,
                                inner: crate::service::client::ProviderInnerConfig::Oauth(
                                    OauthProviderConfig {
                                        client_id: "client-id".to_string(),
                                        client_secret: "client-secret".to_string(),
                                        scopes: Vec::new(),
                                        authorization_url: Url::parse(
                                            "http://localhost/authorization",
                                        )
                                        .unwrap(),
                                        token_url: Url::parse(&format!(
                                            "http://{}/api/token",
                                            mock.host_with_port()
                                        ))
                                        .unwrap(),
                                        api_user_url: Url::parse("http://localhost/api/user")
                                            .unwrap(),
                                    },
                                ),
                            },
                        )]
                        .into_iter()
                        .collect(),
                    ),
                },
            )]
            .into_iter()
            .collect(),
        );

        let server = Server::new(settings).await;

        let mut tx = server.database.begin().await.unwrap();

        let application = FindApplicationByClientId::new("main-client-id")
            .execute(&mut tx)
            .await
            .unwrap()
            .unwrap();
        let providers = ListProviderByApplicationId::new(application.id)
            .execute(&mut tx)
            .await
            .unwrap();

        let app_state = CsrfToken::new_random().secret().to_owned();
        let (app_code_challenge, app_code_verifier) =
            oauth2::PkceCodeChallenge::new_random_sha256();
        let app_request_id = CreateApplicationAuthorizationRequest::new(
            application.id,
            app_code_challenge.as_str(),
            app_code_challenge.method().as_str(),
            app_state.as_ref(),
            &application.redirect_uri,
        )
        .execute(&mut tx)
        .await
        .unwrap();

        let provider_csrf_token = CsrfToken::new_random();
        let (provider_code_challenge, provider_pkce_verifier) =
            oauth2::PkceCodeChallenge::new_random_sha256();
        let provider_request_id = CreateProviderAuthorizationRequest::new(
            app_request_id,
            providers[0].id,
            provider_csrf_token.secret(),
            provider_pkce_verifier.secret(),
        )
        .execute(&mut tx)
        .await
        .unwrap();

        let _redirected_id =
            CreateRedirectedRequest::new(provider_request_id, provider_code_challenge.as_str())
                .execute(&mut tx)
                .await
                .unwrap();

        tx.commit().await.unwrap();

        let app = server.router();

        let token_mock = mock
            .mock("POST", "/api/token")
            .with_body(
                r#"{"access_token":"foo","token_type":"bearer","expires_in":42,"refresh_token":null}"#,
            )
            .create_async()
            .await;

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/access-token")
                    .method("POST")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "code": app_code_challenge.as_str(),
                            "code_verifier": app_code_verifier.secret(),
                            "grant_type": "code",
                            "redirect_uri": application.redirect_uri.as_str(),
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = res.status();
        assert_eq!(status, StatusCode::OK);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType> =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(body.expires_in().unwrap().as_secs(), 42);

        token_mock.assert_async().await;
    }
}
