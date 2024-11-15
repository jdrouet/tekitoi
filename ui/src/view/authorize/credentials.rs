use std::borrow::Cow;

use another_html_builder::{Body, Buffer};

const fn email_field() -> crate::component::text_field::Component {
    crate::component::text_field::Component {
        rtype: "email",
        id: "email",
        name: "email",
        label: "Email Address",
        placeholder: "user@example.com",
        required: true,
    }
}

const fn password_field() -> crate::component::text_field::Component {
    crate::component::text_field::Component {
        rtype: "password",
        id: "password",
        name: "password",
        label: "Password",
        placeholder: "Fill in your password",
        required: true,
    }
}

#[derive(Debug)]
pub struct Section {
    target: Cow<'static, str>,
}

impl Section {
    pub fn new(target: impl Into<Cow<'static, str>>) -> Self {
        Self {
            target: target.into(),
        }
    }

    pub fn render<'b, W: std::fmt::Write>(&self, buf: Buffer<W, Body<'b>>) -> Buffer<W, Body<'b>> {
        buf.node("form")
            .attr(("class", "card-body"))
            .attr(("attr-provider", "credentials"))
            .attr(("method", "POST"))
            .attr(("action", self.target.as_ref()))
            .content(|buf| {
                let buf = email_field().render(buf);
                let buf = password_field().render(buf);
                buf.node("button")
                    .attr(("type", "submit"))
                    .attr(("class", "hover_shadow success"))
                    .content(|buf| buf.text("Login"))
            })
    }
}
