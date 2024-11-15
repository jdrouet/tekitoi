use std::borrow::Cow;

use another_html_builder::Buffer;

#[derive(Debug)]
pub struct View {
    message: Cow<'static, str>,
    style_path: Option<&'static str>,
}

impl View {
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
            style_path: None,
        }
    }

    pub fn with_style_path(mut self, style_path: &'static str) -> Self {
        self.style_path = Some(style_path);
        self
    }
}

impl super::View for View {
    fn render(self) -> String {
        Buffer::default()
            .doctype()
            .node("html")
            .attr(("lang", "en"))
            .content(|buf| {
                let buf = crate::component::head::render(buf, self.style_path);
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
