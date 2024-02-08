use super::error::ViewError;
use crate::entity::{AuthorizationError, AuthorizationState, RedirectUri};
use crate::model::provider::{ListProviderByApplicationId, Provider};
use crate::model::{
    application::FindApplicationByClientId,
    application_authorization_request::CreateApplicationAuthorizationRequest,
};
use crate::service::database::DatabasePool;
use axum::extract::Query;
use axum::response::{Html, IntoResponse, Response};
use axum::Extension;
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

pub(crate) enum AuthorizeError {
    View(ViewError),
    Redirect(RedirectUri, AuthorizationError),
}

impl AuthorizeError {
    fn view<T: Into<ViewError>>(value: T) -> Self {
        Self::View(value.into())
    }
}

impl IntoResponse for AuthorizeError {
    fn into_response(self) -> Response {
        match self {
            Self::View(inner) => inner.into_response(),
            Self::Redirect(uri, err) => err.as_redirect(uri.inner()).into_response(),
        }
    }
}

pub(crate) async fn handler(
    Extension(db_pool): Extension<DatabasePool>,
    Query(params): Query<AuthorizationRequest>,
) -> Result<Html<String>, AuthorizeError> {
    let mut tx = db_pool.begin().await.map_err(AuthorizeError::view)?;

    // If the client_id doesn't match any application, then we can just display a "not found" page.
    let application = FindApplicationByClientId::new(&params.client_id)
        .execute(&mut tx)
        .await
        .map_err(AuthorizeError::view)?;
    let Some(application) = application else {
        return Err(AuthorizeError::View(ViewError::not_found(
            "Application not found".into(),
            "There is no application defined with the provided client id.".into(),
        )));
    };

    if !application.is_redirect_uri_matching(params.redirect_uri.as_ref()) {
        return Err(AuthorizeError::Redirect(
            params.redirect_uri,
            AuthorizationError::create_redirect_uri_mismatch().with_state(params.state.inner()),
        ));
    }

    let request_id = CreateApplicationAuthorizationRequest::new(
        application.id,
        params.code_challenge.as_str(),
        params.code_challenge.method().as_str(),
        params.state.as_ref(),
        params.redirect_uri.as_ref(),
    )
    .execute(&mut tx)
    .await
    .map_err(AuthorizeError::view)?;

    let providers = ListProviderByApplicationId::new(application.id)
        .execute(&mut tx)
        .await
        .map_err(AuthorizeError::view)?;

    tx.commit().await.map_err(AuthorizeError::view)?;

    let ctx = AuthorizeTemplate {
        request_id,
        providers,
    };
    let template = ctx.render_once().unwrap();

    Ok(Html(template))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::{settings::Settings, Server};
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::header::LOCATION;
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

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("Application not found"));
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn redirect_uri_mismatch() {
        crate::init_logger();

        let (auth_uri, csrf_token) = {
            let client = oauth2::basic::BasicClient::new(
                oauth2::ClientId::new("main-client-id".into()),
                None,
                oauth2::AuthUrl::new("http://authorize/authorize".into()).unwrap(),
                None,
            )
            .set_redirect_uri(
                oauth2::RedirectUrl::new("http://localhost:4444/api/wrong".into()).unwrap(),
            );
            // Generate a PKCE challenge.
            let (pkce_challenge, _pkce_verifier) = oauth2::PkceCodeChallenge::new_random_sha256();

            // Generate the full authorization URL.
            let (auth_url, csrf_token) = client
                .authorize_url(oauth2::CsrfToken::new_random)
                .set_pkce_challenge(pkce_challenge)
                .url();
            let auth_url = auth_url.to_string();
            let auth_uri = auth_url.strip_prefix("http://authorize").unwrap();
            (auth_uri.to_string(), csrf_token)
        };

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
        assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);

        let header = res
            .headers()
            .get(LOCATION)
            .and_then(|h| String::from_utf8(h.as_bytes().to_vec()).ok())
            .unwrap();
        let location = url::Url::parse(header.as_str()).unwrap();
        assert_eq!(location.host_str().unwrap(), "localhost");
        assert_eq!(location.port().unwrap(), 4444);
        assert_eq!(location.path(), "/api/wrong");
        let qp = location
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        assert_eq!(qp.get("error").unwrap(), "redirect_uri_mismatch");
        assert_eq!(
            qp.get("error_description").unwrap(),
            "The redirect_uri MUST match the registered callback URL for this application."
        );
        assert_eq!(qp.get("state").unwrap(), csrf_token.secret());
    }
}
