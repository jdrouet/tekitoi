use super::error::ApiError;
use super::prelude::AccessToken;
use crate::service::cache::CachePool;
use crate::service::client::ClientManager;
use crate::service::client::ProviderUser;
use axum::{Extension, Json};
use oauth2::TokenResponse;

pub async fn handler(
    AccessToken(token): AccessToken,
    Extension(clients): Extension<ClientManager>,
    Extension(cache): Extension<CachePool>,
) -> Result<Json<ProviderUser>, ApiError> {
    tracing::trace!("user with token={:?}", token);
    let mut cache_conn = cache.acquire().await?;
    let access_token = cache_conn
        .find_provider_access_token(token.as_str())
        .await?;
    let Some(access_token) = access_token else {
        return Err(ApiError::bad_request("authentication request not found"));
    };
    tracing::debug!("access token found");
    //
    let user = clients
        .get(access_token.client_id.as_str())
        .map_err(ApiError::internal_server)?
        .providers
        .get(access_token.kind.as_str())
        .ok_or_else(|| ApiError::internal_server("Provider not found."))?
        .get_api_client(access_token.inner.access_token().secret().as_str())
        .fetch_user()
        .await
        .map_err(ApiError::internal_server)?;

    Ok(Json(user))
}
