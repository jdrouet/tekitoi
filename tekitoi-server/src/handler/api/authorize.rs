use super::error::ApiError;
use crate::handler::view::authorize::InitialAuthorizationRequest;
use crate::service::cache::Pool as CachePool;
use crate::service::client::ClientManager;
use actix_web::{get, http::header::LOCATION, web::Data, web::Path, HttpResponse};
use deadpool_redis::redis;
use oauth2::{CsrfToken, PkceCodeChallenge, PkceCodeVerifier};
use serde_qs as qs;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AuthorizationRequest {
    #[serde(flatten)]
    pub initial: InitialAuthorizationRequest,
    pub pkce_verifier: PkceCodeVerifier,
}

impl AuthorizationRequest {
    pub fn to_query_string(&self) -> Result<String, qs::Error> {
        qs::to_string(self)
    }

    pub fn from_query_string(value: &str) -> Result<Self, qs::Error> {
        qs::from_str(value)
    }
}

#[get("/api/authorize/{kind}/{state}")]
async fn handle(
    clients: Data<ClientManager>,
    cache: Data<CachePool>,
    path: Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (kind, state) = path.into_inner();
    tracing::trace!(
        "authorization redirection kind={:?} state={:?}",
        kind,
        state
    );
    let mut cache_conn = cache.get().await?;
    let initial_str: String = redis::cmd("GETDEL")
        .arg(state.as_str())
        .query_async(&mut cache_conn)
        .await?;
    let initial = InitialAuthorizationRequest::from_query_string(&initial_str)?;
    // build oauth client
    let oauth_client = clients
        .get_oauth_client(initial.client_id.as_ref(), kind.as_str())
        .ok_or_else(|| ApiError::BadRequest {
            message: "no client or provider found".into(),
        })?;
    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    // Generate the full authorization URL.
    let (auth_url, csrf_token) = oauth_client
        .authorize_url(CsrfToken::new_random)
        // Set the PKCE code challenge.
        .set_pkce_challenge(pkce_challenge)
        .url();

    let auth_request = AuthorizationRequest {
        initial,
        pkce_verifier,
    };
    let auth_request = auth_request.to_query_string()?;
    redis::cmd("SETEX")
        .arg(csrf_token.secret())
        .arg(60i32 * 10)
        .arg(auth_request)
        .query_async(&mut cache_conn)
        .await?;

    let auth_url = auth_url.to_string();

    tracing::trace!("redirect to {:?} authorization page: {:?}", kind, auth_url);
    Ok(HttpResponse::Found()
        .append_header((LOCATION, auth_url))
        .finish())
}
