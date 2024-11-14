use std::borrow::Cow;

use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};

#[derive(Debug)]
pub(super) struct Error {
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

    fn render(&self) -> String {
        another_html_builder::Buffer::default()
            .doctype()
            .node("html")
            .attr(("lang", "en"))
            .content(|buf| {
                let buf = super::helper::render_head(buf);
                buf.node("body").content(|buf| {
                    buf.node("div")
                        .attr(("class", "card shadow"))
                        .content(|buf| {
                            buf.node("div")
                                .attr(("class", "card-header text-center"))
                                .content(|buf| buf.text("Error"))
                                .node("div")
                                .attr(("class", "card-body"))
                                .content(|buf| buf.text(self.message.as_ref()))
                        })
                })
            })
            .into_inner()
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.status, Html(self.render())).into_response()
    }
}
