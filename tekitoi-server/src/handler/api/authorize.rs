use super::error::ApiError;
use crate::model::provider::FindProviderForInitialRequest;
use crate::model::{local::CreateLocalRequest, provider::ListProviderScopesById};
use crate::service::database::DatabasePool;
use crate::service::BaseUrl;
use axum::{extract::Path, response::Redirect, Extension};
use oauth2::{CsrfToken, PkceCodeChallenge, Scope};
use uuid::Uuid;

pub async fn handler(
    Extension(base_url): Extension<BaseUrl>,
    Extension(pool): Extension<DatabasePool>,
    Path((request_id, provider_id)): Path<(Uuid, Uuid)>,
) -> Result<Redirect, ApiError> {
    let mut tx = pool.begin().await?;

    // build oauth client
    let provider = FindProviderForInitialRequest::new(request_id, provider_id)
        .execute(&mut tx)
        .await?;
    let Some(provider) = provider else {
        return Err(ApiError::bad_request("provider not found"));
    };
    let scopes = ListProviderScopesById::new(provider.id)
        .execute(&mut tx)
        .await?;

    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    // Generate the full authorization URL.
    let client = provider.oauth_client(base_url.as_ref());
    let auth_request = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(scopes.iter().cloned().map(Scope::new));
    let (auth_url, csrf_token) = auth_request.set_pkce_challenge(pkce_challenge).url();

    CreateLocalRequest::new(
        request_id,
        provider_id,
        csrf_token.secret(),
        pkce_verifier.secret(),
    )
    .execute(&mut tx)
    .await?;

    let auth_url = auth_url.to_string();

    tx.commit().await?;

    Ok(Redirect::temporary(&auth_url))
}

// #[cfg(test)]
// mod tests {
//     use crate::model::incoming::IncomingRequest;
//     use crate::{settings::Settings, Server};
//     use axum::body::Body;
//     use axum::extract::Request;
//     use axum::http::header::LOCATION;
//     use axum::http::StatusCode;
//     use http_body_util::BodyExt;
//     use std::path::PathBuf;
//     use tower::util::ServiceExt;

//     fn settings() -> Settings {
//         Settings::build(Some(PathBuf::from("./tests/simple.toml")))
//     }

//     #[tokio::test]
//     async fn unknown_state() {
//         let app = Server::new(settings()).await.router();

//         let res = app
//             .oneshot(
//                 Request::builder()
//                     .uri("/api/authorize/github/whatever")
//                     .method("GET")
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         assert_eq!(res.status(), StatusCode::BAD_REQUEST);

//         let body = res.into_body().collect().await.unwrap().to_bytes();
//         let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
//         assert_eq!(body, serde_json::json!("state not found"));
//     }

//     #[tokio::test]
//     async fn valid_provider() {
//         let server = Server::new(settings());
//         let initial = IncomingRequest {
//             client_id: "main-client-id".into(),
//             code_challenge: "code-challenge".into(),
//             code_challenge_method: "S256".into(),
//             state: "state".into(),
//             redirect_uri: url::Url::parse("http://localhost:4444/api/redirect").unwrap(),
//         };
//         let random_token = oauth2::CsrfToken::new_random();
//         let mut cache_client = server.cache_pool.acquire().await.unwrap();
//         cache_client
//             .insert_incoming_authorization_request(random_token.secret(), initial)
//             .await
//             .unwrap();

//         let app = server.router();
//         let uri = format!("/api/authorize/github/{}", random_token.secret());

//         let res = app
//             .oneshot(
//                 Request::builder()
//                     .uri(&uri)
//                     .method("GET")
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);

//         let location = res.headers().get(LOCATION).unwrap().to_str().unwrap();
//         let location = url::Url::parse(location).unwrap();
//         assert_eq!(location.domain(), Some("github.com"));
//         assert_eq!(location.scheme(), "https");
//         assert_eq!(location.path(), "/login/oauth/authorize");
//     }

//     #[tokio::test]
//     async fn invalid_provider() {
//         let server = Server::new(settings());
//         let initial = IncomingRequest {
//             client_id: "main-client-id".into(),
//             code_challenge: "code-challenge".into(),
//             code_challenge_method: "S256".into(),
//             state: "state".into(),
//             redirect_uri: url::Url::parse("http://localhost:4444/api/redirect").unwrap(),
//         };
//         let random_token = oauth2::CsrfToken::new_random();
//         let mut cache_client = server.cache_pool.acquire().await.unwrap();
//         cache_client
//             .insert_incoming_authorization_request(random_token.secret(), initial)
//             .await
//             .unwrap();

//         let app = server.router();
//         let uri = format!("/api/authorize/unknown/{}", random_token.secret());

//         let res = app
//             .oneshot(
//                 Request::builder()
//                     .uri(&uri)
//                     .method("GET")
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         assert_eq!(res.status(), StatusCode::BAD_REQUEST);

//         let body = res.into_body().collect().await.unwrap().to_bytes();
//         let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
//         assert_eq!(body, serde_json::json!("provider not found"));
//     }
// }
