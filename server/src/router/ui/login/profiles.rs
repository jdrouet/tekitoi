use std::borrow::Cow;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Extension;
use tekitoi_ui::view::View;
use uuid::Uuid;

use crate::entity::provider::ProviderKind;
use crate::helper::generate_token;
use crate::router::ui::authorize::AUTHORIZATION_TTL;
use crate::router::ui::error::Error;
use crate::router::ui::helper::encode_url;

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
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::response::Response {
        Error::new(self.status(), self.message()).into_response()
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[cfg_attr(test, derive(Debug))]
pub(crate) struct QueryParams<'a> {
    #[serde(flatten)]
    pub base: Cow<'a, crate::router::ui::authorize::BaseQueryParams>,
    pub user: Uuid,
}

pub(crate) async fn handle(
    Extension(database): Extension<crate::service::database::Pool>,
    Query(params): Query<QueryParams<'static>>,
) -> Result<Html<String>, ResponseError> {
    let mut tx = database.as_ref().begin().await?;
    let app = crate::entity::application::FindById::new(params.base.client_id)
        .execute(&mut *tx)
        .await?;
    let app = app.ok_or(ResponseError::ApplicationNotFound)?;
    if !app.redirect_uri.eq(params.base.redirect_uri.as_str()) {
        return Err(ResponseError::InvalidRedirectUri);
    }
    let user =
        crate::entity::user::FindByIdAndProvider::new(params.user, app.id, ProviderKind::Profiles)
            .execute(&mut *tx)
            .await?;
    let user = user.ok_or(ResponseError::UserNotFound)?;

    let code = generate_token(24);
    let request = crate::entity::authorization::Create {
        code: code.as_str(),
        state: params.base.state.as_str(),
        scope: params.base.scope.as_deref(),
        code_challenge: params.base.code_challenge.as_str(),
        code_challenge_method: params.base.code_challenge_method, // S256
        response_type: params.base.response_type,                 // code
        client_id: params.base.client_id,
        user_id: user.id,
        time_to_live: AUTHORIZATION_TTL,
    };
    request.execute(&mut *tx).await?;
    tx.commit().await?;

    let redirection_url = encode_url(
        &params.base.redirect_uri,
        [
            ("code", code.as_str()),
            ("state", params.base.state.as_str()),
        ]
        .into_iter(),
    );
    let redirection = tekitoi_ui::view::redirect::View::new(redirection_url).render();
    Ok(Html(redirection))
}
