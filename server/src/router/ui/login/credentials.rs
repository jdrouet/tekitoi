use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum::{Extension, Form};
use tekitoi_ui::view::View;

use crate::entity::user::FindForCredentials;
use crate::helper::generate_token;
use crate::router::ui::authorize::{BaseQueryParams, AUTHORIZATION_TTL};
use crate::router::ui::error::Error;
use crate::router::ui::helper::encode_url;

pub(crate) enum ResponseError {
    ApplicationNotFound,
    InvalidCredentials(BaseQueryParams),
    InvalidRedirectUri,
    Database,
}

impl From<sqlx::Error> for ResponseError {
    fn from(value: sqlx::Error) -> Self {
        tracing::error!(message = "database interaction failed", error = %value);
        Self::Database
    }
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::ApplicationNotFound => Error::new(
                StatusCode::NOT_FOUND,
                "Application not found with provided client ID.",
            )
            .into_response(),
            Self::InvalidRedirectUri => Error::new(
                StatusCode::BAD_REQUEST,
                "The provided redirect URI is invalid.",
            )
            .into_response(),
            Self::Database => {
                Error::new(StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
                    .into_response()
            }
            Self::InvalidCredentials(params) => {
                let params = serde_urlencoded::to_string(&params).unwrap();
                let uri = format!("/authorize?{params}");
                Redirect::temporary(uri.as_str()).into_response()
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct RequestPayload {
    email: String,
    password: String,
}

pub(crate) async fn handle(
    Extension(database): Extension<crate::service::database::Pool>,
    Query(params): Query<BaseQueryParams>,
    Form(payload): Form<RequestPayload>,
) -> Result<Html<String>, ResponseError> {
    let mut tx = database.as_ref().begin().await?;
    let app = crate::entity::application::FindById::new(params.client_id)
        .execute(&mut *tx)
        .await?;
    let app = app.ok_or(ResponseError::ApplicationNotFound)?;
    if !app.redirect_uri.eq(params.redirect_uri.as_str()) {
        return Err(ResponseError::InvalidRedirectUri);
    }

    let user = FindForCredentials::new(app.id, payload.email.as_str())
        .execute(&mut *tx)
        .await?;
    let Some(user) = user else {
        tracing::warn!(message = "user not found with provided email", email = %payload.email);
        return Err(ResponseError::InvalidCredentials(params));
    };
    if !user.check_password(payload.password.as_str()) {
        tracing::warn!(message = "invalid password", email = %payload.email);
        return Err(ResponseError::InvalidCredentials(params));
    }

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
    tx.commit().await?;

    let redirection_url = encode_url(
        &params.redirect_uri,
        [("code", code.as_str()), ("state", params.state.as_str())].into_iter(),
    );
    let redirection = tekitoi_ui::view::redirect::View::new(redirection_url).render();
    Ok(Html(redirection))
}
