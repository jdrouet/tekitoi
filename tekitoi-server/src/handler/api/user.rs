use super::error::ApiError;
use super::prelude::{AccessToken, CachePayload};
use crate::service::cache::Pool as CachePool;
use crate::service::client::ClientManager;
use crate::{handler::api::token::ProviderAccessToken, service::client::ProviderUser};
use axum::{Extension, Json};
use deadpool_redis::redis;
use oauth2::TokenResponse;

// #[get("/api/user")]
pub async fn handler(
    AccessToken(token): AccessToken,
    Extension(clients): Extension<ClientManager>,
    Extension(cache): Extension<CachePool>,
) -> Result<Json<ProviderUser>, ApiError> {
    tracing::trace!("user with token={:?}", token);
    let mut cache_conn = cache.get().await?;
    let auth_request: String = redis::cmd("GET")
        .arg(token)
        .query_async(&mut cache_conn)
        .await?;
    tracing::debug!("access token found");
    let access_token = ProviderAccessToken::from_query_string(&auth_request)?;
    tracing::debug!("access token deserialized");
    //
    let user = clients
        .get(access_token.client_id.as_str())
        .map_err(ApiError::internal_server)?
        .providers
        .get(access_token.kind.as_str())
        .ok_or_else(|| ApiError::InternalServer {
            message: "Provider not found.".into(),
        })?
        .get_api_client(access_token.inner.access_token().secret().as_str())
        .fetch_user()
        .await
        .map_err(|message| ApiError::InternalServer { message })?;
    Ok(Json(user))
}
