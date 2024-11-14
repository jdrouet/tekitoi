use std::borrow::Cow;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

#[derive(Debug, serde::Serialize)]
pub(super) struct Error {
    #[serde(skip)]
    status: StatusCode,
    #[serde(rename = "error")]
    message: Cow<'static, str>,
}

impl Error {
    #[inline]
    pub fn new(status: StatusCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn internal() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "something went wrong")
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self)).into_response()
    }
}
