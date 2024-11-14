use std::borrow::Cow;
use std::collections::HashSet;
use std::time::Duration;

use another_html_builder::{Body, Buffer};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Extension;
use uuid::Uuid;

use crate::entity::code_challenge::CodeChallengeMethod;
use crate::entity::provider::ProviderKind;
use crate::entity::response_type::ResponseType;
use crate::entity::user::Entity as UserEntity;

// 10 mins
pub(super) const AUTHORIZATION_TTL: Duration = Duration::new(600, 0);

pub(super) fn render_head(buf: Buffer<String, Body<'_>>) -> Buffer<String, Body<'_>> {
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
            Self::ApplicationNotFound => StatusCode::NOT_FOUND,
            Self::InvalidRedirectUri => StatusCode::BAD_REQUEST,
            Self::UnableToBuildPage | Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::ApplicationNotFound => "Application not found with provided client ID.",
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

pub(crate) struct UserListSection {
    users: Vec<(String, String)>,
}

impl UserListSection {
    fn new(params: &QueryParams, users: Vec<UserEntity>) -> anyhow::Result<Self> {
        let mut users_generated = Vec::new();
        for user in users {
            let target_params = super::login::userlist::QueryParams {
                parent: Cow::Borrowed(params),
                user: user.id,
            };
            let target_params = serde_urlencoded::to_string(&target_params)?;
            let link = format!(
                "/authorize/{}/login?{target_params}",
                ProviderKind::UserList
            );
            users_generated.push((user.login, link));
        }
        Ok(Self {
            users: users_generated,
        })
    }

    fn render<'b>(&self, buf: Buffer<String, Body<'b>>) -> Buffer<String, Body<'b>> {
        buf.node("div").attr(("class", "list")).content(|buf| {
            self.users.iter().fold(buf, |buf, (login, link)| {
                buf.node("a")
                    .attr(("class", "list-item"))
                    .attr(("href", link.as_str()))
                    .content(|buf| buf.text("Login as ").text(login.as_str()))
            })
        })
    }
}

#[derive(Default)]
pub(crate) struct ResponseSuccess {
    pub user_list: Option<UserListSection>,
}

impl ResponseSuccess {
    fn render_body<'b>(&self, buf: Buffer<String, Body<'b>>) -> Buffer<String, Body<'b>> {
        buf.node("body").content(|buf| {
            buf.node("main")
                .attr(("class", "card shadow"))
                .content(|buf| {
                    let buf = buf
                        .node("div")
                        .attr(("class", "card-header text-center"))
                        .content(|buf| buf.text("Authentication"));
                    self.user_list
                        .iter()
                        .fold(buf, |buf, section| section.render(buf))
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

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[cfg_attr(test, derive(Debug))]
pub(crate) struct QueryParams {
    pub client_id: Uuid,
    pub redirect_uri: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: CodeChallengeMethod, // S256
    pub response_type: ResponseType,                // code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
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

    let mut success = ResponseSuccess::default();
    let providers = crate::entity::provider::ListByApplication::new(app.id)
        .execute(&mut *tx)
        .await?;
    let providers: HashSet<_> = providers.into_iter().map(|p| p.kind).collect();

    if providers.contains(&ProviderKind::UserList) {
        let users =
            crate::entity::user::ListForApplicationAndProvider::new(app.id, ProviderKind::UserList)
                .execute(&mut *tx)
                .await?;
        success.user_list = Some(UserListSection::new(&params, users).map_err(|err| {
            tracing::error!(message = "unable to generate user list section", source = %err);
            ResponseError::UnableToBuildPage
        })?);
    }

    tx.commit().await?;

    Ok(Html(success.render()))
}

#[cfg(test)]
mod tests {}
