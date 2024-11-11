use axum::http::StatusCode;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use super::error::Error;

pub(super) struct AuthorizationToken(pub Bearer);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthorizationToken
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        if let Ok(TypedHeader(Authorization(inner))) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await
        {
            Ok(AuthorizationToken(inner))
        } else {
            Err(Error::new(
                StatusCode::UNAUTHORIZED,
                "unable to get authorization token",
            ))
        }
    }
}
