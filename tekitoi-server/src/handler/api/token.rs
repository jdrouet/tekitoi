use super::error::ApiError;
use crate::model::local::GetLocalRequestById;
use crate::model::provider::GetProviderById;
use crate::model::redirected::FindRedirectedRequestByCode;
use crate::model::token::CreateAccessToken;
use crate::service::database::DatabasePool;
use crate::service::BaseUrl;
use axum::{Extension, Form, Json};
use chrono::Duration;
use oauth2::basic::BasicTokenType;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AccessToken, AuthorizationCode, EmptyExtraTokenFields, PkceCodeVerifier, StandardTokenResponse,
    TokenResponse,
};
use url::Url;

#[derive(Debug, serde::Deserialize)]
pub struct TokenRequestPayload {
    pub grant_type: String,
    pub code_verifier: String,
    pub redirect_uri: Url,
    pub code: String,
    #[serde(flatten)]
    pub others: serde_json::Value,
}

pub async fn handler(
    Extension(base_url): Extension<BaseUrl>,
    Extension(pool): Extension<DatabasePool>,
    Form(payload): Form<TokenRequestPayload>,
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
    let local_request = GetLocalRequestById::new(redirected_request.local_request_id)
        .execute(&mut tx)
        .await?;

    let provider = GetProviderById::new(local_request.provider_id)
        .execute(&mut tx)
        .await?;
    //
    let oauth_client = provider.oauth_client(base_url.as_ref());
    // Now you can trade it for an access token.
    let token_result = oauth_client
        .exchange_code(AuthorizationCode::new(redirected_request.code))
        // Set the PKCE code verifier.
        .set_pkce_verifier(PkceCodeVerifier::new(local_request.pkce_verifier))
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
            ApiError::internal_server(err)
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
    let token_response = StandardTokenResponse::<EmptyExtraTokenFields, BasicTokenType>::new(
        AccessToken::new(token_id.to_string()),
        BasicTokenType::Bearer,
        EmptyExtraTokenFields {},
    );

    tx.commit().await?;
    //
    Ok(Json(token_response))
}
