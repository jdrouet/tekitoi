use super::error::ApiError;
use crate::model::incoming::GetIncomingRequestById;
use crate::model::local::FindLocalRequestByState;
use crate::model::redirected::CreateRedirectedRequest;
use crate::service::database::DatabasePool;
use axum::extract::Query;
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
    Extension(pool): Extension<DatabasePool>,
    Query(query): Query<QueryParams>,
) -> Result<Redirect, ApiError> {
    let mut tx = pool.begin().await?;

    let local = FindLocalRequestByState::new(query.state())
        .execute(&mut tx)
        .await?;
    let Some(local) = local else {
        return Err(ApiError::bad_request("unable to find request"));
    };

    let initial = GetIncomingRequestById::new(local.initial_request_id)
        .execute(&mut tx)
        .await?;

    let query = match query {
        QueryParams::Ok(value) => value,
        QueryParams::Error(value) => {
            tracing::debug!(
                "something went wrong with provider {:?}",
                value.error_description
            );
            let url = merge_url(&initial.redirect_uri, &value)?;
            return Ok(Redirect::temporary(&url));
        }
    };
    let code_challenge = initial.code_challenge.clone();
    let state = initial.state.clone();
    let redirect_uri = initial.redirect_uri.clone();

    let response_query = QueryParamsOk {
        state,
        code: code_challenge.as_str().to_string(),
    };
    let url = merge_url(&redirect_uri, &response_query)?;

    // TODO find what to store before having token request
    CreateRedirectedRequest::new(local.id, &query.code)
        .execute(&mut tx)
        .await?;

    tx.commit().await?;
    //
    Ok(Redirect::temporary(&url))
}
