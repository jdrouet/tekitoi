use super::error::ViewError;
use crate::handler::api::prelude::CachePayload;
use crate::service::cache::Pool as CachePool;
use crate::service::client::ClientManager;
use actix_web::http::header::ContentType;
use actix_web::{get, web::Data, web::Query, HttpResponse};
use deadpool_redis::redis;
use oauth2::CsrfToken;
use sailfish::TemplateOnce;
use url::Url;

// response_type=code
// client_id=
// code_challenge=
// code_challenge_method=
// state=
// redirect_uri=

// TODO add response_type with an enum
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InitialAuthorizationRequest {
    pub client_id: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub state: String,
    pub redirect_uri: Url,
}

impl CachePayload for InitialAuthorizationRequest {}

#[derive(TemplateOnce)]
#[template(path = "authorize.html")]
struct AuthorizeTemplate<'a> {
    state: &'a str,
    providers: Vec<&'static str>,
}

#[get("/authorize")]
async fn handle(
    clients: Data<ClientManager>,
    cache: Data<CachePool>,
    params: Query<InitialAuthorizationRequest>,
) -> Result<HttpResponse, ViewError> {
    tracing::trace!("authorization page requested");
    let client = clients
        .get_client(params.client_id.as_str())
        .map_err(|err| {
            ViewError::bad_request("Invalid authorization request".into(), err.into())
        })?;
    client
        .check_redirect_uri(&params.redirect_uri)
        .map_err(|err| {
            ViewError::bad_request("Invalid authorization request".into(), err.into())
        })?;
    let csrf_token = CsrfToken::new_random();
    let mut cache_conn = cache.get().await?;
    redis::cmd("SETEX")
        .arg(csrf_token.secret())
        .arg(60i32 * 10)
        .arg(params.to_query_string()?)
        .query_async(&mut cache_conn)
        .await?;
    let ctx = AuthorizeTemplate {
        state: csrf_token.secret(),
        providers: client.providers.names(),
    };
    let template = ctx.render_once().unwrap();
    Ok(HttpResponse::Ok()
        .insert_header(ContentType::html())
        .body(template))
}

#[cfg(test)]
mod tests {
    use crate::tests::TestServer;
    use actix_web::http::StatusCode;

    #[actix_web::test]
    async fn basic() {
        let client = oauth2::basic::BasicClient::new(
            oauth2::ClientId::new("main-client-id".into()),
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
            // Set the desired scopes.
            // .add_scope(Scope::new("read".to_string()))
            // .add_scope(Scope::new("write".to_string()))
            // Set the PKCE code challenge.
            .set_pkce_challenge(pkce_challenge)
            .url();
        let auth_url = auth_url.to_string();
        let auth_uri = auth_url.strip_prefix("http://authorize").unwrap();
        let req = actix_web::test::TestRequest::get()
            .uri(auth_uri)
            .to_request();
        let srv = TestServer::from_simple();
        let res = srv.execute(req).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = actix_web::test::read_body(res).await;
        let payload = std::str::from_utf8(&body).unwrap();
        assert!(payload.contains("github"));
        let re = regex::Regex::new("\"/api/authorize/github/(.*)\"").unwrap();
        assert!(re.find(payload).is_some());
    }

    #[actix_web::test]
    async fn client_not_found() {
        let client = oauth2::basic::BasicClient::new(
            oauth2::ClientId::new("unknown-client-id".into()),
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
            // Set the desired scopes.
            // .add_scope(Scope::new("read".to_string()))
            // .add_scope(Scope::new("write".to_string()))
            // Set the PKCE code challenge.
            .set_pkce_challenge(pkce_challenge)
            .url();
        let auth_url = auth_url.to_string();
        let auth_uri = auth_url.strip_prefix("http://authorize").unwrap();
        let req = actix_web::test::TestRequest::get()
            .uri(auth_uri)
            .to_request();
        let srv = TestServer::from_simple();
        let res = srv.execute(req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = actix_web::test::read_body(res).await;
        let payload = std::str::from_utf8(&body).unwrap();
        assert!(payload.contains("Client not found."));
    }
}
