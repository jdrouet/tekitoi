use super::error::ApiError;
use super::prelude::CachePayload;
use crate::handler::view::authorize::InitialAuthorizationRequest;
use crate::service::cache::Pool as CachePool;
use crate::service::client::ClientManager;
use axum::{extract::Path, response::Redirect, Extension};
use deadpool_redis::redis;
use oauth2::{CsrfToken, PkceCodeChallenge, PkceCodeVerifier};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AuthorizationRequest {
    pub initial: InitialAuthorizationRequest,
    pub pkce_verifier: PkceCodeVerifier,
}

impl CachePayload for AuthorizationRequest {}

// #[get("/api/authorize/{kind}/{state}")]
pub async fn handler(
    Extension(clients): Extension<ClientManager>,
    Extension(cache): Extension<CachePool>,
    Path((kind, state)): Path<(String, String)>,
) -> Result<Redirect, ApiError> {
    tracing::trace!(
        "authorization redirection kind={:?} state={:?}",
        kind,
        state
    );
    let mut cache_conn = cache.get().await?;
    let initial_str: Option<String> = redis::cmd("GETDEL")
        .arg(state.as_str())
        .query_async(&mut cache_conn)
        .await?;
    let initial_str = initial_str.ok_or_else(|| ApiError::bad_request("state not found"))?;
    let initial = InitialAuthorizationRequest::from_query_string(&initial_str)?;
    // build oauth client
    let client = clients
        .get(initial.client_id.as_str())
        .map_err(ApiError::bad_request)?;
    let provider = client
        .providers
        .get(kind.as_str())
        .ok_or_else(|| ApiError::bad_request("provider not found"))?;
    let oauth_client = provider.get_oauth_client();
    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    // Generate the full authorization URL.
    let (auth_url, csrf_token) = provider
        .with_oauth_scopes(oauth_client.authorize_url(CsrfToken::new_random))
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
    Ok(Redirect::temporary(&auth_url))
}

// #[cfg(test)]
// mod tests {
//     use crate::handler::api::prelude::CachePayload;
//     use crate::handler::view::authorize::InitialAuthorizationRequest;
//     use crate::tests::TestServer;
//     use actix_web::http::{header::LOCATION, StatusCode};
//     use deadpool_redis::redis;
//     use url::Url;

//     #[actix_web::test]
//     async fn unknown_state() {
//         let req = actix_web::test::TestRequest::get()
//             .uri("/api/authorize/github/whatever")
//             .to_request();
//         let srv = TestServer::from_simple();
//         let res = srv.execute(req).await;
//         assert_eq!(res.status(), StatusCode::BAD_REQUEST);
//         let body: String = actix_web::test::read_body_json(res).await;
//         assert_eq!(body, "state not found");
//     }

//     #[actix_web::test]
//     async fn valid_provider() {
//         let srv = TestServer::from_simple();
//         let initial = InitialAuthorizationRequest {
//             client_id: "main-client-id".into(),
//             code_challenge: "code-challenge".into(),
//             code_challenge_method: "S256".into(),
//             state: "state".into(),
//             redirect_uri: Url::parse("http://localhost:4444/api/redirect").unwrap(),
//         };
//         let random_token = oauth2::CsrfToken::new_random();
//         let mut cache_conn = srv.cache_pool.get().await.unwrap();
//         let _: Option<String> = redis::cmd("SETEX")
//             .arg(random_token.secret())
//             .arg(60i32 * 10)
//             .arg(initial.to_query_string().unwrap())
//             .query_async(&mut cache_conn)
//             .await
//             .unwrap();
//         let uri = format!("/api/authorize/github/{}", random_token.secret());
//         let req = actix_web::test::TestRequest::get()
//             .uri(uri.as_str())
//             .to_request();
//         let res = srv.execute(req).await;
//         assert_eq!(res.status(), StatusCode::FOUND);
//         let location = res.headers().get(LOCATION).unwrap().to_str().unwrap();
//         let location = Url::parse(location).unwrap();
//         assert_eq!(location.domain(), Some("github.com"));
//         assert_eq!(location.scheme(), "https");
//         assert_eq!(location.path(), "/login/oauth/authorize");
//     }

//     #[actix_web::test]
//     async fn invalid_provider() {
//         let srv = TestServer::from_simple();
//         let initial = InitialAuthorizationRequest {
//             client_id: "main-client-id".into(),
//             code_challenge: "code-challenge".into(),
//             code_challenge_method: "S256".into(),
//             state: "state".into(),
//             redirect_uri: Url::parse("http://localhost:4444/api/redirect").unwrap(),
//         };
//         let random_token = oauth2::CsrfToken::new_random();
//         let mut cache_conn = srv.cache_pool.get().await.unwrap();
//         let _: Option<String> = redis::cmd("SETEX")
//             .arg(random_token.secret())
//             .arg(60i32 * 10)
//             .arg(initial.to_query_string().unwrap())
//             .query_async(&mut cache_conn)
//             .await
//             .unwrap();
//         let uri = format!("/api/authorize/unknown/{}", random_token.secret());
//         let req = actix_web::test::TestRequest::get()
//             .uri(uri.as_str())
//             .to_request();
//         let res = srv.execute(req).await;
//         assert_eq!(res.status(), StatusCode::BAD_REQUEST);
//         let body: String = actix_web::test::read_body_json(res).await;
//         assert_eq!(body, "provider not found");
//     }
// }
