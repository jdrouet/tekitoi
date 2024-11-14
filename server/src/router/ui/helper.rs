use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;

use another_html_builder::{Body, Buffer};

pub(super) fn redirection<T: Display>(target: T) -> String {
    let content = format!("1; url='{target}'");
    another_html_builder::Buffer::default()
        .doctype()
        .node("html")
        .attr(("lang", "en"))
        .content(|buf| {
            buf.node("head")
                .content(|buf| {
                    buf.node("meta")
                        .attr(("http-equiv", "refresh"))
                        .attr(("content", content.as_str()))
                        .close()
                })
                .node("body")
                .content(|buf| {
                    buf.node("p")
                        .content(|buf| buf.text("You will be redirected soon..."))
                })
        })
        .into_inner()
}

pub(super) fn encode_params<'a>(
    values: impl Iterator<Item = (&'a str, &'a str)>,
) -> Option<String> {
    let params = HashMap::<&str, &str>::from_iter(values);
    if params.is_empty() {
        None
    } else {
        serde_urlencoded::to_string(&params).ok()
    }
}

pub(super) fn encode_url<'a>(
    path: &'a str,
    params: impl Iterator<Item = (&'a str, &'a str)>,
) -> Cow<'a, str> {
    match encode_params(params) {
        Some(values) => Cow::Owned(format!("{path}?{values}")),
        None => Cow::Borrowed(path),
    }
}

pub(super) fn render_head(buf: Buffer<String, Body<'_>>) -> Buffer<String, Body<'_>> {
    buf.node("head").content(|buf| {
        buf.node("meta")
            .attr(("charset", "utf-8"))
            .close()
            .node("meta")
            .attr(("name", "viewport"))
            .attr(("content", "width=device-width, initial-scale=1"))
            .close()
            .node("title")
            .content(|buf| buf.text("🔑 Authorization"))
            .node("link")
            .attr(("rel", "stylesheet"))
            .attr(("href", crate::router::asset::STYLE_PATH))
            .close()
    })
}
