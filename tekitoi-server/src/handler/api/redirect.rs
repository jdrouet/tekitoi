use super::error::ApiError;
use crate::model::application_authorization_request::GetApplicationAuthorizationRequestById;
use crate::model::provider_authorization_request::FindProviderAuthorizationRequestByState;
use crate::model::redirected::CreateRedirectedRequest;
use crate::service::database::DatabasePool;
use axum::extract::Query;
use axum::response::Redirect;
use axum::Extension;
use serde_qs as qs;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct QueryParamsOk {
    pub code: String,
    pub state: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct QueryParamsError {
    pub error: String,
    pub error_description: String,
    pub error_uri: String,
    pub state: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum QueryParams {
    Ok(QueryParamsOk),
    Error(QueryParamsError),
}

impl QueryParams {
    fn state(&self) -> &str {
        match self {
            Self::Ok(value) => value.state.as_str(),
            Self::Error(value) => value.state.as_str(),
        }
    }
}

fn merge_url<S: serde::Serialize>(url: &url::Url, params: &S) -> Result<String, qs::Error> {
    let queries = qs::to_string(params)?;
    Ok(format!("{}?{}", url, queries))
}

pub(crate) async fn handler(
    Extension(pool): Extension<DatabasePool>,
    Query(query): Query<QueryParams>,
) -> Result<Redirect, ApiError> {
    let mut tx = pool.begin().await?;

    let provider_request = FindProviderAuthorizationRequestByState::new(query.state())
        .execute(&mut tx)
        .await?;
    let Some(provider_request) = provider_request else {
        return Err(ApiError::bad_request("unable to find request"));
    };

    let app_request = GetApplicationAuthorizationRequestById::new(
        provider_request.application_authorization_request_id,
    )
    .execute(&mut tx)
    .await?;

    let query = match query {
        QueryParams::Ok(value) => value,
        QueryParams::Error(value) => {
            tracing::debug!(
                "something went wrong with provider {:?}",
                value.error_description
            );
            let url = merge_url(&app_request.redirect_uri, &value)?;
            return Ok(Redirect::temporary(&url));
        }
    };
    let code_challenge = app_request.code_challenge.clone();
    let state = app_request.state.clone();
    let redirect_uri = app_request.redirect_uri.clone();

    let response_query = QueryParamsOk {
        state,
        code: code_challenge.as_str().to_string(),
    };
    let url = merge_url(&redirect_uri, &response_query)?;

    // TODO find what to store before having token request
    CreateRedirectedRequest::new(provider_request.id, &query.code)
        .execute(&mut tx)
        .await?;

    tx.commit().await?;
    //
    Ok(Redirect::temporary(&url))
}

#[cfg(test)]
mod tests {
    use crate::model::application::FindApplicationByClientId;
    use crate::model::application_authorization_request::CreateApplicationAuthorizationRequest;
    use crate::model::provider::ListProviderByApplicationId;
    use crate::model::provider_authorization_request::CreateProviderAuthorizationRequest;
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

    fn settings() -> Settings {
        Settings::build(Some(PathBuf::from("./tests/simple.toml")))
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn with_success() {
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

        let app_state = CsrfToken::new_random().secret().to_owned();
        let (app_code_challenge, _app_pkce_verifier) =
            oauth2::PkceCodeChallenge::new_random_sha256();
        let app_request_id = CreateApplicationAuthorizationRequest::new(
            application.id,
            app_code_challenge.as_str(),
            app_code_challenge.method().as_str(),
            app_state.as_ref(),
            &application.redirect_uri,
        )
        .execute(&mut tx)
        .await
        .unwrap();

        let provider_csrf_token = CsrfToken::new_random();
        let (_provider_code_challenge, provider_pkce_verifier) =
            oauth2::PkceCodeChallenge::new_random_sha256();
        CreateProviderAuthorizationRequest::new(
            app_request_id,
            providers[0].id,
            provider_csrf_token.secret(),
            provider_pkce_verifier.secret(),
        )
        .execute(&mut tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let app = server.router();

        let res = app
            .oneshot(
                Request::builder()
                    .uri(&format!(
                        "/api/redirect?code=foo&state={}",
                        provider_csrf_token.secret()
                    ))
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = res.status();
        assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);

        let location = res.headers().get(LOCATION).unwrap();
        let location = location.to_str().unwrap().to_owned();
        let location = url::Url::parse(&location).unwrap();
        assert_eq!(location.host_str().unwrap(), "localhost");
        let query = location.query_pairs().collect::<HashMap<_, _>>();
        println!("query: {query:?}");
        assert_eq!(query.get("code").unwrap(), app_code_challenge.as_str());
        assert_eq!(query.get("state").unwrap(), app_state.as_str());
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn unknown_state() {
        crate::init_logger();

        let app = Server::new(settings()).await.router();

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/redirect?code=foo&state=bar")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body, serde_json::json!("unable to find request"));
    }
}
