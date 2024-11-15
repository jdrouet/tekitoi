use another_html_builder::{Body, Buffer};

pub mod credentials;
pub mod profiles;

#[derive(Default)]
pub struct View {
    profiles: Option<profiles::Section>,
    credentials: Option<credentials::Section>,
    style_path: Option<&'static str>,
}

impl View {
    pub fn set_credentials(&mut self, section: credentials::Section) {
        self.credentials = Some(section);
    }

    pub fn set_profiles(&mut self, section: profiles::Section) {
        self.profiles = Some(section);
    }

    pub fn with_style_path(mut self, style_path: &'static str) -> Self {
        self.style_path = Some(style_path);
        self
    }

    fn render_body<'b, W: std::fmt::Write>(&self, buf: Buffer<W, Body<'b>>) -> Buffer<W, Body<'b>> {
        buf.node("body").content(|buf| {
            buf.node("main")
                .attr(("class", "card shadow"))
                .content(|buf| {
                    let buf = buf
                        .node("div")
                        .attr(("class", "card-header text-center"))
                        .content(|buf| buf.text("Authentication"));
                    let buf = self
                        .profiles
                        .iter()
                        .fold(buf, |buf, section| section.render(buf));
                    let buf = if self.profiles.is_some() && self.credentials.is_some() {
                        buf.node("hr").attr(("class", "separator")).close()
                    } else {
                        buf
                    };
                    self.credentials
                        .iter()
                        .fold(buf, |buf, section| section.render(buf))
                })
        })
    }
}

impl crate::view::View for View {
    fn render(self) -> String {
        Buffer::default()
            .doctype()
            .node("html")
            .attr(("lang", "en"))
            .content(|buf| {
                let buf = crate::component::head::render(buf, self.style_path);
                self.render_body(buf)
            })
            .into_inner()
    }
}
