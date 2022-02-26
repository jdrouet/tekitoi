use super::error::ApiError;
use super::prelude::CachePayload;
use crate::handler::api::authorize::AuthorizationRequest;
use crate::service::cache::Pool as CachePool;
use crate::service::client::ClientManager;
use actix_web::http::header::LOCATION;
use actix_web::web::{Data, Path, Query};
use actix_web::{get, HttpResponse};
use deadpool_redis::redis;
use serde_qs as qs;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RedirectedAuthorizationRequest {
    pub inner: AuthorizationRequest,
    pub code: String,
    pub kind: String,
}

impl CachePayload for RedirectedAuthorizationRequest {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryParamsOk {
    pub code: String,
    pub state: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryParamsError {
    pub error: String,
    pub error_description: String,
    pub error_uri: String,
    pub state: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum QueryParams {
    Ok(QueryParamsOk),
    Error(QueryParamsError),
}

impl QueryParams {
    pub fn state(&self) -> &str {
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

#[get("/api/redirect/{kind}")]
async fn handle(
    _clients: Data<ClientManager>,
    cache: Data<CachePool>,
    path: Path<String>,
    query: Query<QueryParams>,
) -> Result<HttpResponse, ApiError> {
    tracing::trace!("redirection requested");
    let kind = path.into_inner();
    let mut cache_conn = cache.get().await?;
    let auth_request: String = redis::cmd("GETDEL")
        .arg(query.state())
        .query_async(&mut cache_conn)
        .await?;
    let auth_request = AuthorizationRequest::from_query_string(&auth_request)?;
    let query = match query.into_inner() {
        QueryParams::Ok(value) => value,
        QueryParams::Error(value) => {
            tracing::debug!(
                "something went wrong with provider {:?}",
                value.error_description
            );
            let url = merge_url(&auth_request.initial.redirect_uri, &value)?;
            return Ok(HttpResponse::Found()
                .append_header((LOCATION, url))
                .finish());
        }
    };
    let code_challenge = auth_request.initial.code_challenge.clone();
    let state = auth_request.initial.state.clone();
    let redirect_uri = auth_request.initial.redirect_uri.clone();
    let response_query = QueryParamsOk {
        state,
        code: code_challenge.as_str().to_string(),
        // code: auth_request.pkce_verifier.secret().to_string(),
    };
    let url = merge_url(&redirect_uri, &response_query)?;
    // TODO find what to store before having token request
    let redirect_request = RedirectedAuthorizationRequest {
        inner: auth_request,
        code: query.code.clone(),
        kind,
    };
    let redirect_request = redirect_request.to_query_string()?;
    let _ = redis::cmd("SETEX")
        .arg(code_challenge.as_str())
        .arg(60i32 * 10)
        .arg(redirect_request.as_str())
        .query_async(&mut cache_conn)
        .await?;
    tracing::debug!("redirecting to {:?}", url);
    //
    Ok(HttpResponse::Found()
        .append_header((LOCATION, url))
        .finish())
}
