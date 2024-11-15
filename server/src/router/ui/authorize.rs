use std::borrow::Cow;
use std::collections::HashSet;
use std::time::Duration;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Extension;
use tekitoi_ui::view::View;
use uuid::Uuid;

use crate::entity::code_challenge::CodeChallengeMethod;
use crate::entity::provider::ProviderKind;
use crate::entity::response_type::ResponseType;
use crate::entity::user::Entity as UserEntity;

// 10 mins
pub(super) const AUTHORIZATION_TTL: Duration = Duration::new(600, 0);

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
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::response::Response {
        super::error::Error::new(self.status(), self.message()).into_response()
    }
}

fn credentials_section(
    params: &QueryParams,
) -> anyhow::Result<tekitoi_ui::view::authorize::credentials::Section> {
    let params = serde_urlencoded::to_string(params)?;
    let target = format!("/authorize/{}/login?{params}", ProviderKind::Credentials);
    Ok(tekitoi_ui::view::authorize::credentials::Section::new(
        target,
    ))
}

fn profiles_section(
    params: &QueryParams,
    users: Vec<UserEntity>,
) -> anyhow::Result<tekitoi_ui::view::authorize::profiles::Section> {
    let mut res = tekitoi_ui::view::authorize::profiles::Section::default();
    for user in users {
        let target_params = super::login::profiles::QueryParams {
            parent: Cow::Borrowed(params),
            user: user.id,
        };
        let target_params = serde_urlencoded::to_string(&target_params)?;
        let link = format!(
            "/authorize/{}/login?{target_params}",
            ProviderKind::Profiles
        );
        res.add_user(user.login, link);
    }
    Ok(res)
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

    let mut success = tekitoi_ui::view::authorize::View::default();
    let providers = crate::entity::provider::ListByApplication::new(app.id)
        .execute(&mut *tx)
        .await?;
    let providers: HashSet<_> = providers.into_iter().map(|p| p.kind).collect();

    if providers.contains(&ProviderKind::Profiles) {
        let users =
            crate::entity::user::ListForApplicationAndProvider::new(app.id, ProviderKind::Profiles)
                .execute(&mut *tx)
                .await?;
        let section = profiles_section(&params, users).map_err(|err| {
            tracing::error!(message = "unable to generate profiles section", source = %err);
            ResponseError::UnableToBuildPage
        })?;
        success.set_profiles(section);
    }

    if providers.contains(&ProviderKind::Credentials) {
        let section = credentials_section(&params).map_err(|err| {
            tracing::error!(message = "unable to generate credentials section", source = %err);
            ResponseError::UnableToBuildPage
        })?;
        success.set_credentials(section);
    }

    tx.commit().await?;

    Ok(Html(success.render()))
}
