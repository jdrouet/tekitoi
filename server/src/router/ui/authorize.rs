use std::time::Duration;

use another_html_builder::{Body, Buffer};
use anyhow::Context;
use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension,
};
use uuid::Uuid;

use crate::{
    entity::{
        code_challenge::CodeChallengeMethod, response_type::ResponseType,
        user::Entity as UserEntity,
    },
    helper::generate_token,
    router::ui::helper::{encode_url, redirection},
};

// 10 mins
const AUTHORIZATION_TTL: Duration = Duration::new(600, 0);

fn render_head<'b>(buf: Buffer<String, Body<'b>>) -> Buffer<String, Body<'b>> {
    buf.node("head").content(|buf| {
        buf.node("meta")
            .attr(("charset", "utf-8"))
            .close()
            .node("meta")
            .attr(("name", "viewport"))
            .attr(("content", "width=device-width, initial-scale=1"))
            .close()
            .node("title")
            .content(|buf| buf.text("🔑 Authorization"))
            .node("link")
            .attr(("rel", "stylesheet"))
            .attr(("href", crate::router::asset::STYLE_PATH))
            .close()
    })
}

pub(crate) enum ResponseError {
    ApplicationNotFound,
    UserNotFound,
    InvalidRedirectUri,
    UnableToBuildPage,
    Database,
}

impl From<sqlx::Error> for ResponseError {
    fn from(value: sqlx::Error) -> Self {
        tracing::error!(message = "database interaction failed", error = %value);
        Self::Database
    }
}

impl ResponseError {
    fn status(&self) -> StatusCode {
        match self {
            Self::ApplicationNotFound | Self::UserNotFound => StatusCode::NOT_FOUND,
            Self::InvalidRedirectUri => StatusCode::BAD_REQUEST,
            Self::UnableToBuildPage | Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::ApplicationNotFound => "Application not found with provided client ID.",
            Self::UserNotFound => "User not found with provided client ID.",
            Self::InvalidRedirectUri => "The provided redirect URI is invalid.",
            Self::UnableToBuildPage | Self::Database => "Something went wrong...",
        }
    }

    fn render(&self) -> String {
        another_html_builder::Buffer::default()
            .doctype()
            .node("html")
            .attr(("lang", "en"))
            .content(|buf| {
                let buf = render_head(buf);
                buf.node("body").content(|buf| {
                    buf.node("div")
                        .attr(("class", "card shadow"))
                        .content(|buf| {
                            buf.node("div")
                                .attr(("class", "card-header text-center"))
                                .content(|buf| buf.text("Error"))
                                .node("div")
                                .attr(("class", "card-body"))
                                .content(|buf| buf.text(self.message()))
                        })
                })
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
    fn new(
        mut params: QueryParams,
        users: impl Iterator<Item = &'a UserEntity>,
    ) -> anyhow::Result<Self> {
        let mut users_generated = Vec::new();
        for user in users {
            params.user = Some(user.id);
            let link = params.to_url()?;
            users_generated.push((user.login.as_str(), link));
        }
        Ok(Self {
            users: users_generated,
        })
    }

    fn render_body<'b>(&self, buf: Buffer<String, Body<'b>>) -> Buffer<String, Body<'b>> {
        buf.node("body").content(|buf| {
            buf.node("main")
                .attr(("class", "card shadow"))
                .content(|buf| {
                    let buf = buf
                        .node("div")
                        .attr(("class", "card-header text-center"))
                        .content(|buf| buf.text("Authentication"));
                    buf.node("div").attr(("class", "list")).content(|buf| {
                        self.users.iter().fold(buf, |buf, (login, link)| {
                            buf.node("a")
                                .attr(("class", "list-item"))
                                .attr(("href", link.as_str()))
                                .content(|buf| buf.text("Login as ").text(login))
                        })
                    })
                })
        })
    }

    fn render(&self) -> String {
        another_html_builder::Buffer::default()
            .doctype()
            .node("html")
            .attr(("lang", "en"))
            .content(|buf| {
                let buf = render_head(buf);
                self.render_body(buf)
            })
            .into_inner()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(test, derive(Debug))]
pub(crate) struct QueryParams {
    client_id: Uuid,
    redirect_uri: String,
    state: String,
    code_challenge: String,
    code_challenge_method: CodeChallengeMethod, // S256
    response_type: ResponseType,                // code
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<Uuid>,
}

impl QueryParams {
    fn to_url(&self) -> anyhow::Result<String> {
        let params = serde_urlencoded::to_string(self).context("url encoding params")?;
        Ok(format!("/authorize?{params}"))
    }
}

pub(super) async fn handle(
    Extension(database): Extension<crate::service::database::Pool>,
    Query(params): Query<QueryParams>,
) -> Result<Html<String>, ResponseError> {
    let mut tx = database.as_ref().begin().await?;
    let app = crate::entity::application::FindById::new(params.client_id)
        .execute(&mut *tx)
        .await?;
    let app = app.ok_or(ResponseError::ApplicationNotFound)?;
    if !app.redirect_uri.eq(params.redirect_uri.as_str()) {
        return Err(ResponseError::InvalidRedirectUri);
    }
    let html = match params.user {
        Some(user_id) => {
            let user = crate::entity::user::FindById::new(user_id, app.id)
                .execute(&mut *tx)
                .await?;
            let user = user.ok_or(ResponseError::UserNotFound)?;

            let code = generate_token(24);
            let request = crate::entity::authorization::Create {
                code: code.as_str(),
                state: params.state.as_str(),
                scope: params.scope.as_deref(),
                code_challenge: params.code_challenge.as_str(),
                code_challenge_method: params.code_challenge_method, // S256
                response_type: params.response_type,                 // code
                client_id: params.client_id,
                user_id: user.id,
                time_to_live: AUTHORIZATION_TTL,
            };
            request.execute(&mut *tx).await?;
            let redirection_url = encode_url(
                &params.redirect_uri,
                [("code", code.as_str()), ("state", params.state.as_str())].into_iter(),
            );
            redirection(redirection_url)
        }
        None => {
            let users = crate::entity::user::ListForApplication::new(app.id)
                .execute(&mut *tx)
                .await?;
            ResponseSuccess::new(params, users.iter())
                .map_err(|err| {
                    tracing::error!(message = "unable to generate page", source = %err);
                    ResponseError::UnableToBuildPage
                })?
                .render()
        }
    };
    tx.commit().await?;
    Ok(Html(html))
}

#[cfg(test)]
mod tests {}
