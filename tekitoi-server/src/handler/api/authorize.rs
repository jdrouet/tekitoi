use super::error::ApiError;
use crate::entity::local::LocalAuthorizationRequest;
use crate::service::cache::CachePool;
use crate::service::client::ClientManager;
use axum::{extract::Path, response::Redirect, Extension};
use oauth2::{CsrfToken, PkceCodeChallenge};

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
    let mut cache_conn = cache.acquire().await?;
    let initial = cache_conn
        .remove_incoming_authorization_request(state.as_str())
        .await?;
    let Some(initial) = initial else {
        return Err(ApiError::bad_request("state not found"));
    };
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

    let auth_request = LocalAuthorizationRequest {
        initial,
        pkce_verifier,
    };
    cache_conn
        .insert_local_authorization_request(csrf_token.secret(), auth_request)
        .await?;

    let auth_url = auth_url.to_string();

    tracing::trace!("redirect to {:?} authorization page: {:?}", kind, auth_url);
    Ok(Redirect::temporary(&auth_url))
}

#[cfg(test)]
mod tests {
    use crate::entity::incoming::IncomingAuthorizationRequest;
    use crate::{settings::Settings, Server};
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::header::LOCATION;
    use axum::http::StatusCode;
    use http_body_util::BodyExt;
    use std::path::PathBuf;
    use tower::util::ServiceExt;

    fn settings() -> Settings {
        Settings::build(Some(PathBuf::from("./tests/simple.toml")))
    }

    #[tokio::test]
    async fn unknown_state() {
        let app = Server::new(settings()).router();

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/authorize/github/whatever")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body, serde_json::json!("state not found"));
    }

    #[tokio::test]
    async fn valid_provider() {
        let server = Server::new(settings());
        let initial = IncomingAuthorizationRequest {
            client_id: "main-client-id".into(),
            code_challenge: "code-challenge".into(),
            code_challenge_method: "S256".into(),
            state: "state".into(),
            redirect_uri: url::Url::parse("http://localhost:4444/api/redirect").unwrap(),
        };
        let random_token = oauth2::CsrfToken::new_random();
        let mut cache_client = server.cache_pool.acquire().await.unwrap();
        cache_client
            .insert_incoming_authorization_request(random_token.secret(), initial)
            .await
            .unwrap();

        let app = server.router();
        let uri = format!("/api/authorize/github/{}", random_token.secret());

        let res = app
            .oneshot(
                Request::builder()
                    .uri(&uri)
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);

        let location = res.headers().get(LOCATION).unwrap().to_str().unwrap();
        let location = url::Url::parse(location).unwrap();
        assert_eq!(location.domain(), Some("github.com"));
        assert_eq!(location.scheme(), "https");
        assert_eq!(location.path(), "/login/oauth/authorize");
    }

    #[tokio::test]
    async fn invalid_provider() {
        let server = Server::new(settings());
        let initial = IncomingAuthorizationRequest {
            client_id: "main-client-id".into(),
            code_challenge: "code-challenge".into(),
            code_challenge_method: "S256".into(),
            state: "state".into(),
            redirect_uri: url::Url::parse("http://localhost:4444/api/redirect").unwrap(),
        };
        let random_token = oauth2::CsrfToken::new_random();
        let mut cache_client = server.cache_pool.acquire().await.unwrap();
        cache_client
            .insert_incoming_authorization_request(random_token.secret(), initial)
            .await
            .unwrap();

        let app = server.router();
        let uri = format!("/api/authorize/unknown/{}", random_token.secret());

        let res = app
            .oneshot(
                Request::builder()
                    .uri(&uri)
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body, serde_json::json!("provider not found"));
    }
}
