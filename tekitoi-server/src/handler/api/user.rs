use super::error::ApiError;
use super::prelude::CachePayload;
use crate::handler::api::token::ProviderAccessToken;
use crate::service::cache::Pool as CachePool;
use crate::service::client::ClientManager;
use actix_web::web::Data;
use actix_web::{get, HttpRequest, HttpResponse};
use deadpool_redis::redis;
use oauth2::TokenResponse;

fn get_access_token<'a>(req: &'a HttpRequest) -> Result<&'a str, ApiError> {
    req.headers()
        .get("authorization")
        .ok_or_else(|| ApiError::BadRequest {
            message: "Authorization header not found.".into(),
        })?
        .to_str()
        .map_err(|error| {
            tracing::debug!("invalid authorization header: {:?}", error);
            ApiError::BadRequest {
                message: "Unable to read authorization header.".into(),
            }
        })
        .and_then(|token| {
            token.strip_prefix("Bearer ").ok_or_else(|| {
                tracing::debug!("invalid authorization header format");
                ApiError::BadRequest {
                    message: "Invalid authorization header format.".into(),
                }
            })
        })
}

#[get("/api/user")]
async fn handle(
    clients: Data<ClientManager>,
    cache: Data<CachePool>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let token = get_access_token(&req)?;
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
        .get_client(access_token.client_id.as_str())
        .ok_or_else(|| ApiError::InternalServer {
            message: "Client not found.".into(),
        })?
        .providers
        .get(access_token.kind.as_str())
        .ok_or_else(|| ApiError::InternalServer {
            message: "Provider not found.".into(),
        })?
        .get_api_client(access_token.inner.access_token().secret().as_str())
        .fetch_user()
        .await
        .map_err(|message| ApiError::InternalServer { message })?;
    Ok(HttpResponse::Ok().json(user))
}
