use std::borrow::Cow;

use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension,
};
use uuid::Uuid;

use crate::{
    entity::provider::ProviderKind,
    helper::generate_token,
    router::ui::{
        authorize::{render_head, AUTHORIZATION_TTL},
        helper::{encode_url, redirection},
    },
};

pub(crate) enum ResponseError {
    ApplicationNotFound,
    UserNotFound,
    InvalidRedirectUri,
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
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::ApplicationNotFound => "Application not found with provided client ID.",
            Self::UserNotFound => "User not found with provided client ID.",
            Self::InvalidRedirectUri => "The provided redirect URI is invalid.",
            Self::Database => "Something went wrong...",
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

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[cfg_attr(test, derive(Debug))]
pub(crate) struct QueryParams<'a> {
    #[serde(flatten)]
    pub parent: Cow<'a, crate::router::ui::authorize::QueryParams>,
    pub user: Uuid,
}

pub(crate) async fn handle(
    Extension(database): Extension<crate::service::database::Pool>,
    Query(params): Query<QueryParams<'static>>,
) -> Result<Html<String>, ResponseError> {
    let mut tx = database.as_ref().begin().await?;
    let app = crate::entity::application::FindById::new(params.parent.client_id)
        .execute(&mut *tx)
        .await?;
    let app = app.ok_or(ResponseError::ApplicationNotFound)?;
    if !app.redirect_uri.eq(params.parent.redirect_uri.as_str()) {
        return Err(ResponseError::InvalidRedirectUri);
    }
    let user =
        crate::entity::user::FindByIdAndProvider::new(params.user, app.id, ProviderKind::UserList)
            .execute(&mut *tx)
            .await?;
    let user = user.ok_or(ResponseError::UserNotFound)?;

    let code = generate_token(24);
    let request = crate::entity::authorization::Create {
        code: code.as_str(),
        state: params.parent.state.as_str(),
        scope: params.parent.scope.as_deref(),
        code_challenge: params.parent.code_challenge.as_str(),
        code_challenge_method: params.parent.code_challenge_method, // S256
        response_type: params.parent.response_type,                 // code
        client_id: params.parent.client_id,
        user_id: user.id,
        time_to_live: AUTHORIZATION_TTL,
    };
    request.execute(&mut *tx).await?;
    tx.commit().await?;

    let redirection_url = encode_url(
        &params.parent.redirect_uri,
        [
            ("code", code.as_str()),
            ("state", params.parent.state.as_str()),
        ]
        .into_iter(),
    );
    Ok(Html(redirection(redirection_url)))
}
