use std::borrow::Cow;

use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use tekitoi_ui::view::View;

#[derive(Debug)]
pub(super) struct Error {
    status: StatusCode,
    view: tekitoi_ui::view::error::View,
}

impl Error {
    #[inline]
    pub fn new(status: StatusCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            status,
            view: tekitoi_ui::view::error::View::new(message),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.status, Html(self.view.render())).into_response()
    }
}
