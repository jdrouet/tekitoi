use super::error::ViewError;
use crate::entity::{AuthorizationState, RedirectUri};
use crate::model::provider::{ListProviderByApplicationId, Provider};
use crate::model::{
    application::FindApplicationByClientId,
    application_authorization_request::CreateApplicationAuthorizationRequest,
};
use crate::service::database::DatabasePool;
use axum::{extract::Query, response::Html, Extension};
use oauth2::{ClientId, PkceCodeChallenge};
use sailfish::TemplateOnce;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct AuthorizationRequest {
    client_id: ClientId,
    #[serde(flatten)]
    code_challenge: PkceCodeChallenge,
    state: AuthorizationState,
    redirect_uri: RedirectUri,
}

#[derive(TemplateOnce)]
#[template(path = "authorize.html")]
struct AuthorizeTemplate {
    request_id: Uuid,
    providers: Vec<Provider>,
}

pub(crate) async fn handler(
    Extension(db_pool): Extension<DatabasePool>,
    Query(params): Query<AuthorizationRequest>,
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
        .check_redirect_uri(params.redirect_uri.as_ref())
        .map_err(|err| {
            ViewError::bad_request("Invalid authorization request".into(), err.into())
        })?;

    let request_id = CreateApplicationAuthorizationRequest::new(
        application.id,
        params.code_challenge.as_str(),
        params.code_challenge.method().as_str(),
        params.state.as_ref(),
        params.redirect_uri.as_ref(),
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
    #[serial_test::serial(database)]
    async fn simple() {
        crate::init_logger();

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

        let status = res.status();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&body[..]);

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Connect with github"));

        let re = regex::Regex::new("\"/api/authorize/(.*)/(.*)\"").unwrap();
        assert!(re.find(&body).is_some());
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn client_not_found() {
        crate::init_logger();

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

        let status = res.status();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&body[..]);

        println!("body: {body}");

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains("Application not found"));
    }
}
