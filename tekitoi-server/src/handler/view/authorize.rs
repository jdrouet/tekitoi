use super::error::ViewError;
use crate::service::cache::Pool as CachePool;
use crate::service::client::ClientManager;
use actix_web::http::header::ContentType;
use actix_web::{get, web::Data, web::Query, HttpResponse};
use deadpool_redis::redis;
use oauth2::CsrfToken;
use sailfish::TemplateOnce;
use serde_qs as qs;
use url::Url;

// response_type=code
// client_id=
// code_challenge=
// code_challenge_method=
// state=
// redirect_uri=

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InitialAuthorizationRequest {
    pub client_id: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub state: String,
    pub redirect_uri: Url,
}

impl InitialAuthorizationRequest {
    pub fn to_query_string(&self) -> Result<String, qs::Error> {
        qs::to_string(self)
    }

    pub fn from_query_string(value: &str) -> Result<Self, qs::Error> {
        qs::from_str(value)
    }
}

#[derive(TemplateOnce)]
#[template(path = "authorize.html")]
struct AuthorizeTemplate<'a> {
    state: &'a str,
}

#[get("/authorize")]
async fn handle(
    clients: Data<ClientManager>,
    cache: Data<CachePool>,
    params: Query<InitialAuthorizationRequest>,
) -> Result<HttpResponse, ViewError> {
    tracing::trace!("authorization page requested");
    if let Err(msg) = clients.validate(params.client_id.as_str(), &params.redirect_uri) {
        tracing::debug!("invalid pair client_id/redirect_uri: {:?}", msg);
        return Err(ViewError::bad_request(
            "Invalid authorization request".into(),
            msg.into(),
        ));
    }
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
    };
    let template = ctx.render_once().unwrap();
    Ok(HttpResponse::Ok()
        .insert_header(ContentType::html())
        .body(template))
}
