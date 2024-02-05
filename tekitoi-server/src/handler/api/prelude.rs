use std::str::FromStr;

use axum::http::{header::AUTHORIZATION, request::Parts};
use uuid::Uuid;

use super::error::ApiError;

pub(crate) struct AccessToken(pub Uuid);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AccessToken
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get(AUTHORIZATION)
            .ok_or_else(|| ApiError::unauthorized("Authorization header not found."))
            .and_then(|h| {
                h.to_str()
                    .map_err(|_| ApiError::unauthorized("Unable to read authorization header."))
            })
            .and_then(|v| {
                v.strip_prefix("Bearer ")
                    .ok_or_else(|| ApiError::unauthorized("Invalid authorization header format."))
            })
            .and_then(|v| {
                Uuid::from_str(v).map_err(|_| ApiError::unauthorized("Invalid token content."))
            })
            .map(Self)
    }
}
