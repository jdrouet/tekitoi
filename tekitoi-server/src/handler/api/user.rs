use super::error::ApiError;
use super::prelude::AccessToken;
use crate::model::provider::GetProviderByAccessToken;
use crate::service::database::DatabasePool;
use crate::{model::token::FindAccessToken, service::client::ProviderUser};
use axum::{Extension, Json};

#[axum::debug_handler]
pub(crate) async fn handler(
    Extension(pool): Extension<DatabasePool>,
    AccessToken(token): AccessToken,
) -> Result<Json<ProviderUser>, ApiError> {
    let mut tx = pool.begin().await?;

    let access_token = FindAccessToken::new(token).execute(&mut tx).await?;
    let Some(access_token) = access_token else {
        tracing::debug!("unable to find access token with token={token:?}");
        return Err(ApiError::bad_request("authentication request not found"));
    };
    tracing::debug!("access token found");

    let provider = GetProviderByAccessToken::new(token)
        .execute(&mut tx)
        .await?;

    let user = provider
        .config
        .provider_client(access_token.token)
        .fetch_user()
        .await
        .map_err(ApiError::internal_server)?;

    Ok(Json(user))
}
