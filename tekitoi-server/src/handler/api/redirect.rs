use super::error::ApiError;
use crate::entity::redirected::RedirectedAuthorizationRequest;
use crate::service::cache::CachePool;
use axum::extract::{Path, Query};
use axum::response::Redirect;
use axum::Extension;
use serde_qs as qs;

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

pub async fn handler(
    Extension(cache): Extension<CachePool>,
    Path(kind): Path<String>,
    Query(query): Query<QueryParams>,
) -> Result<Redirect, ApiError> {
    tracing::trace!("redirection requested");
    let mut cache_conn = cache.acquire().await?;
    let auth_request = cache_conn
        .remove_local_authorization_request(query.state())
        .await?;
    let Some(auth_request) = auth_request else {
        return Err(ApiError::bad_request(
            "unable to find authorization request",
        ));
    };
    let query = match query {
        QueryParams::Ok(value) => value,
        QueryParams::Error(value) => {
            tracing::debug!(
                "something went wrong with provider {:?}",
                value.error_description
            );
            let url = merge_url(&auth_request.initial.redirect_uri, &value)?;
            return Ok(Redirect::temporary(&url));
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
        code: query.code,
        kind,
    };
    cache_conn
        .insert_redirected_authorization_request(code_challenge.as_str(), redirect_request)
        .await?;
    tracing::debug!("redirecting to {:?}", url);
    //
    Ok(Redirect::temporary(&url))
}
