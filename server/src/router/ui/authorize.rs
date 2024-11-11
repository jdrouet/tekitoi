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

    fn render(&self) -> String {
        another_html_builder::Buffer::default()
            .doctype()
            .node("html")
            .attr(("lang", "en"))
            .content(|buf| {
                buf.node("head")
                    .content(|buf| {
                        buf.node("meta")
                            .attr(("charset", "utf-8"))
                            .close()
                            .node("meta")
                            .attr(("name", "viewport"))
                            .attr(("content", "width=device-width, initial-scale=1"))
                            .close()
                    })
                    .node("body")
                    .content(|buf| buf.node("div").content(|buf| buf.text(self.message())))
            })
            .into_inner()
    }
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::response::Response {
        (self.status(), Html(self.render())).into_response()
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

    fn render(&self) -> String {
        another_html_builder::Buffer::default()
            .doctype()
            .node("html")
            .attr(("lang", "en"))
            .content(|buf| {
                buf.node("head")
                    .content(|buf| {
                        buf.node("meta")
                            .attr(("charset", "utf-8"))
                            .close()
                            .node("meta")
                            .attr(("name", "viewport"))
                            .attr(("content", "width=device-width, initial-scale=1"))
                            .close()
                    })
                    .node("body")
                    .content(|buf| {
                        buf.node("div").content(|buf| {
                            self.users.iter().fold(buf, |buf, (login, link)| {
                                buf.node("p").content(|buf| {
                                    buf.node("a")
                                        .attr(("href", link.as_str()))
                                        .content(|buf| buf.text(login))
                                })
                            })
                        })
                    })
            })
            .into_inner()
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
                    &AuthorizationState::new(
                        params.state.clone(),
                        params.scope,
                        params.client_id,
                        user.id,
                    ),
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
            .render(),
    };
    Ok(Html(html))
}

#[cfg(test)]
mod tests {}
