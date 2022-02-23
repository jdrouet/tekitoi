use super::error::ApiError;
use super::prelude::CachePayload;
use crate::handler::api::redirect::RedirectedAuthorizationRequest;
use crate::service::cache::Pool as CachePool;
use crate::service::client::ClientManager;
use actix_web::web::{Data, Form};
use actix_web::{post, HttpResponse};
use deadpool_redis::redis;
use oauth2::basic::BasicTokenType;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AccessToken, AuthorizationCode, EmptyExtraTokenFields, StandardTokenResponse, TokenResponse,
};
use url::Url;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProviderAccessToken {
    pub inner: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    pub kind: String,
    pub client_id: String,
}

impl CachePayload for ProviderAccessToken {}

#[derive(Debug, serde::Deserialize)]
pub struct TokenRequestPayload {
    pub grant_type: String,
    pub code_verifier: String,
    pub redirect_uri: Url,
    pub code: String,
}

#[post("/api/access-token")]
async fn handle(
    clients: Data<ClientManager>,
    cache: Data<CachePool>,
    payload: Form<TokenRequestPayload>,
) -> Result<HttpResponse, ApiError> {
    let payload = payload.into_inner();
    tracing::trace!("access-token requested with code={:?}", payload.code);
    let mut cache_conn = cache.get().await?;
    let auth_request: String = redis::cmd("GETDEL")
        .arg(payload.code.as_str())
        .query_async(&mut cache_conn)
        .await?;
    let auth_request = RedirectedAuthorizationRequest::from_query_string(&auth_request)?;
    tracing::trace!("received authorization request");
    //
    let kind = auth_request.kind;
    let code = auth_request.code;
    let client_id = auth_request.inner.initial.client_id;
    let pkce_verifier = auth_request.inner.pkce_verifier;
    let oauth_client = clients
        .get_client(client_id.as_str())
        .map_err(ApiError::internal_server)?
        .providers
        .get(kind.as_str())
        .ok_or_else(|| ApiError::internal_server("Unable to find provider"))?
        .get_oauth_client();
    // Now you can trade it for an access token.
    let token_result = oauth_client
        .exchange_code(AuthorizationCode::new(code))
        // Set the PKCE code verifier.
        .set_pkce_verifier(pkce_verifier)
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
    tracing::trace!("received token");
    let token_result = ProviderAccessToken {
        inner: token_result,
        client_id,
        kind,
    };
    let token_result_str = token_result.to_query_string()?;
    //
    let token_response = StandardTokenResponse::<EmptyExtraTokenFields, BasicTokenType>::new(
        AccessToken::new(Uuid::new_v4().to_string()),
        BasicTokenType::Bearer,
        EmptyExtraTokenFields {},
    );
    // TODO limit the storage duration based on the token expiration
    let _ = redis::cmd("SET")
        .arg(token_response.access_token().secret())
        .arg(token_result_str.as_str())
        .query_async(&mut cache_conn)
        .await?;
    //
    Ok(HttpResponse::Ok().json(token_response))
}
