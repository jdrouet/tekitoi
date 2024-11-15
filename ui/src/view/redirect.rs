use std::borrow::Cow;

use another_html_builder::{AttributeValue, Buffer};

#[derive(Debug, Clone)]
struct RedirectValue<'a> {
    target: Cow<'a, str>,
}

impl AttributeValue for RedirectValue<'_> {
    fn render(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO escape potential double quotes
        write!(f, "\"1; url='{}'\"", self.target)
    }
}

#[derive(Debug)]
pub struct View<'a> {
    value: RedirectValue<'a>,
}

impl<'a> View<'a> {
    pub fn new(target: Cow<'a, str>) -> Self {
        Self {
            value: RedirectValue { target },
        }
    }
}

impl super::View for View<'_> {
    fn render(self) -> String {
        Buffer::default()
            .doctype()
            .node("html")
            .attr(("lang", "en"))
            .content(|buf| {
                buf.node("head")
                    .content(|buf| {
                        buf.node("meta")
                            .attr(("http-equiv", "refresh"))
                            .attr(("content", self.value))
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
}
