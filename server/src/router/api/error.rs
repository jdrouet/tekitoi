use std::borrow::Cow;

use axum::{http::StatusCode, response::IntoResponse, Json};

#[derive(Debug, serde::Serialize)]
pub(super) struct Error {
    #[serde(skip)]
    status: StatusCode,
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
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self)).into_response()
    }
}
