use anyhow::Context;
use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension,
};
use uuid::Uuid;

use crate::{
    entity::user::Entity as UserEntity,
    router::ui::helper::{encode_url, redirection},
};

pub(crate) enum ResponseError {
    ApplicationNotFound,
    InvalidRedirectUri,
    UnableToBuildPage,
}

impl ResponseError {
    fn status(&self) -> StatusCode {
        match self {
            Self::ApplicationNotFound => StatusCode::NOT_FOUND,
            Self::InvalidRedirectUri => StatusCode::BAD_REQUEST,
            Self::UnableToBuildPage => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::ApplicationNotFound => "Application not found with provided client ID",
            Self::InvalidRedirectUri => "The provided redirect URI is invalid",
            Self::UnableToBuildPage => "Something went wrong...",
        }
    }
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        super::helper::doctype(f)?;
        f.write_str("<html lang=\"en\">")?;
        f.write_str("<head>")?;
        f.write_str("<meta charset=\"utf-8\" />")?;
        f.write_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />")?;
        f.write_str("</head>")?;
        f.write_str("<body>")?;
        f.write_str("<div>")?;
        f.write_str(self.message())?;
        f.write_str("</div>")?;
        f.write_str("</body>")?;
        f.write_str("</html>")
    }
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::response::Response {
        (self.status(), Html(self.to_string())).into_response()
    }
}

pub(crate) struct ResponseSuccess<'a> {
    users: Vec<(&'a str, String)>,
}

impl<'a> ResponseSuccess<'a> {
    fn new(mut params: QueryParams, users: &'a [UserEntity]) -> anyhow::Result<Self> {
        let mut users_generated = Vec::with_capacity(users.len());
        for user in users {
            params.user = Some(user.id);
            let link = params.into_url()?;
            users_generated.push((user.login.as_str(), link));
        }
        Ok(Self {
            users: users_generated,
        })
    }
}

impl<'a> std::fmt::Display for ResponseSuccess<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        super::helper::doctype(f)?;
        f.write_str("<html lang=\"en\">")?;
        f.write_str("<head>")?;
        f.write_str("<meta charset=\"utf-8\" />")?;
        f.write_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />")?;
        f.write_str("</head>")?;
        f.write_str("<body>")?;
        f.write_str("<div>")?;
        for (login, link) in self.users.iter() {
            f.write_str("<p>")?;
            write!(f, "<a href=\"{link}\">Login with {login}</a>")?;
            f.write_str("</p>")?;
        }
        f.write_str("</div>")?;
        f.write_str("</body>")?;
        f.write_str("</html>")
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct QueryParams {
    client_id: String,
    redirect_uri: String,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<Uuid>,
}

impl QueryParams {
    fn into_url(&self) -> anyhow::Result<String> {
        let params = serde_urlencoded::to_string(self).context("url encoding params")?;
        Ok(format!("/authorize?{params}"))
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct AuthorizationState {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub client_id: String,
    pub user: Uuid,
}

impl AuthorizationState {
    pub fn new(state: String, scope: Option<String>, client_id: String, user: Uuid) -> Self {
        Self {
            state,
            scope,
            client_id,
            user,
        }
    }

    pub fn serialize(&self) -> String {
        serde_urlencoded::to_string(self).unwrap()
    }

    pub fn deserialize(input: &str) -> Self {
        serde_urlencoded::from_str(input).unwrap()
    }
}

pub(super) async fn handle(
    Extension(cache): Extension<crate::service::cache::Client>,
    Extension(dataset): Extension<crate::service::dataset::Client>,
    Query(params): Query<QueryParams>,
) -> Result<Html<String>, ResponseError> {
    let app = dataset
        .find(params.client_id.as_str())
        .ok_or(ResponseError::ApplicationNotFound)?;
    if !app.check_redirect_uri(params.redirect_uri.as_str()) {
        return Err(ResponseError::InvalidRedirectUri);
    }
    let html = match params
        .user
        .and_then(|user_id| app.users().iter().find(|user| user.id == user_id))
    {
        Some(user) => {
            let code = uuid::Uuid::new_v4().to_string();
            cache
                .insert(
                    code.clone(),
                    AuthorizationState::new(
                        params.state.clone(),
                        params.scope,
                        params.client_id,
                        user.id,
                    )
                    .serialize(),
                )
                .await;
            let redirection_url = encode_url(
                &params.redirect_uri,
                [("code", code.as_str()), ("state", params.state.as_str())].into_iter(),
            );
            redirection(redirection_url)
        }
        None => ResponseSuccess::new(params, app.users())
            .map_err(|err| {
                tracing::error!(message = "unable to generate page", source = %err);
                ResponseError::UnableToBuildPage
            })?
            .to_string(),
    };
    Ok(Html(html))
}

#[cfg(test)]
mod tests {}
