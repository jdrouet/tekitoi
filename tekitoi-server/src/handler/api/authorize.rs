use super::error::ApiError;
use crate::model::provider::FindProviderForApplicationAuthorizationRequest;
use crate::model::provider_authorization_request::CreateProviderAuthorizationRequest;
use crate::service::database::DatabasePool;
use crate::service::BaseUrl;
use axum::{extract::Path, response::Redirect, Extension};
use uuid::Uuid;

pub(crate) async fn handler(
    Extension(base_url): Extension<BaseUrl>,
    Extension(pool): Extension<DatabasePool>,
    Path((request_id, provider_id)): Path<(Uuid, Uuid)>,
) -> Result<Redirect, ApiError> {
    let mut tx = pool.begin().await?;

    // build oauth client
    let provider = FindProviderForApplicationAuthorizationRequest::new(request_id, provider_id)
        .execute(&mut tx)
        .await?;
    let Some(provider) = provider else {
        return Err(ApiError::bad_request("provider not found"));
    };

    let (auth_url, csrf_token, pkce_verifier) =
        provider.oauth_authorization_request(base_url.as_ref());

    CreateProviderAuthorizationRequest::new(
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

#[cfg(test)]
mod tests {
    use crate::model::application::FindApplicationByClientId;
    use crate::model::application_authorization_request::CreateApplicationAuthorizationRequest;
    use crate::model::provider::ListProviderByApplicationId;
    use crate::{settings::Settings, Server};
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::header::LOCATION;
    use axum::http::StatusCode;
    use http_body_util::BodyExt;
    use oauth2::CsrfToken;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tower::util::ServiceExt;
    use uuid::Uuid;

    fn settings() -> Settings {
        Settings::build(Some(PathBuf::from("./tests/simple.toml")))
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn unknown_request_id() {
        crate::init_logger();

        let app = Server::new(settings()).await.router();

        let request_id = Uuid::new_v4();
        let provider_id = Uuid::new_v4();

        let res = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/authorize/{request_id}/{provider_id}"))
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

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn valid_provider() {
        crate::init_logger();

        let server = Server::new(settings()).await;

        let mut tx = server.database.begin().await.unwrap();
        let application = FindApplicationByClientId::new("main-client-id")
            .execute(&mut tx)
            .await
            .unwrap()
            .unwrap();
        let providers = ListProviderByApplicationId::new(application.id)
            .execute(&mut tx)
            .await
            .unwrap();

        let state = CsrfToken::new_random().secret().to_owned();
        let (code_challenge, _pkce_verifier) = oauth2::PkceCodeChallenge::new_random_sha256();
        let request_id = CreateApplicationAuthorizationRequest::new(
            application.id,
            code_challenge.as_str(),
            code_challenge.method().as_str(),
            state.as_ref(),
            &application.redirect_uri,
        )
        .execute(&mut tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let app = server.router();
        let uri = format!("/api/authorize/{}/{}", request_id, providers[0].id);

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

        let status = res.status();
        let location = res.headers().get(LOCATION).unwrap();

        assert_eq!(status, 307);
        let location = location.to_str().unwrap().to_owned();
        let location = url::Url::parse(&location).unwrap();
        assert_eq!(location.host_str().unwrap(), "github.com");
        let query = location.query_pairs().collect::<HashMap<_, _>>();
        assert_eq!(query.get("response_type").unwrap(), "code");
        assert_eq!(query.get("client_id").unwrap(), "github-client-id");
        assert!(query.contains_key("state"));
        assert!(query.contains_key("code_challenge"));
        assert_eq!(query.get("code_challenge_method").unwrap(), "S256");
        assert_eq!(
            query.get("redirect_uri").unwrap(),
            "http://127.0.0.1:3000/api/redirect"
        );
    }
}
