use super::error::ViewError;
use crate::model::provider::{ListProviderByApplicationId, Provider};
use crate::model::{application::FindApplicationByClientId, incoming::CreateIncomingRequest};
use crate::service::database::DatabasePool;
use axum::{extract::Query, response::Html, Extension};
use sailfish::TemplateOnce;
use url::Url;
use uuid::Uuid;

// response_type=code
// client_id=
// code_challenge=
// code_challenge_method=
// state=
// redirect_uri=

// TODO add response_type with an enum
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryParams {
    pub client_id: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub state: String,
    pub redirect_uri: Url,
}

#[derive(TemplateOnce)]
#[template(path = "authorize.html")]
struct AuthorizeTemplate {
    request_id: Uuid,
    providers: Vec<Provider>,
}

pub async fn handler(
    Extension(db_pool): Extension<DatabasePool>,
    Query(params): Query<QueryParams>,
) -> Result<Html<String>, ViewError> {
    let mut tx = db_pool.begin().await?;

    let application = FindApplicationByClientId::new(&params.client_id)
        .execute(&mut tx)
        .await?;
    let Some(application) = application else {
        return Err(ViewError::bad_request(
            "Application not found".into(),
            "There is no application defined with the provided client id.".into(),
        ));
    };

    application
        .check_redirect_uri(&params.redirect_uri)
        .map_err(|err| {
            ViewError::bad_request("Invalid authorization request".into(), err.into())
        })?;

    let request_id = CreateIncomingRequest::new(
        application.id,
        params.code_challenge.as_str(),
        params.code_challenge_method.as_str(),
        params.state.as_str(),
        &params.redirect_uri,
    )
    .execute(&mut tx)
    .await?;

    let providers = ListProviderByApplicationId::new(application.id)
        .execute(&mut tx)
        .await?;

    tx.commit().await?;

    let ctx = AuthorizeTemplate {
        request_id,
        providers,
    };
    let template = ctx.render_once().unwrap();

    Ok(Html(template))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{settings::Settings, Server};
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use http_body_util::BodyExt;
    use tower::util::ServiceExt;

    fn settings() -> Settings {
        Settings::build(Some(PathBuf::from("./tests/simple.toml")))
    }

    fn create_auth_uri(client_id: &str) -> String {
        let client = oauth2::basic::BasicClient::new(
            oauth2::ClientId::new(client_id.into()),
            None,
            oauth2::AuthUrl::new("http://authorize/authorize".into()).unwrap(),
            None,
        )
        .set_redirect_uri(
            oauth2::RedirectUrl::new("http://localhost:4444/api/redirect".into()).unwrap(),
        );
        // Generate a PKCE challenge.
        let (pkce_challenge, _pkce_verifier) = oauth2::PkceCodeChallenge::new_random_sha256();

        // Generate the full authorization URL.
        let (auth_url, _csrf_token) = client
            .authorize_url(oauth2::CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge)
            .url();
        let auth_url = auth_url.to_string();
        let auth_uri = auth_url.strip_prefix("http://authorize").unwrap();
        auth_uri.to_string()
    }

    #[tokio::test]
    async fn simple() {
        let auth_uri = create_auth_uri("main-client-id");

        let app = Server::new(settings()).await.router();

        let res = app
            .oneshot(
                Request::builder()
                    .uri(auth_uri)
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&body[..]);
        assert!(body.contains("Connect with github"));

        let re = regex::Regex::new("\"/api/authorize/github/(.*)\"").unwrap();
        assert!(re.find(&body).is_some());
    }

    #[tokio::test]
    async fn client_not_found() {
        let auth_uri = create_auth_uri("unknown-client-id");

        let app = Server::new(settings()).await.router();

        let res = app
            .oneshot(
                Request::builder()
                    .uri(auth_uri)
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&body[..]);
        assert!(body.contains("Client not found."));
    }
}
