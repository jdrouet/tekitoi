use another_html_builder::{Body, Buffer};

#[derive(Debug, Default)]
pub struct Section {
    users: Vec<(String, String)>,
}

impl Section {
    pub fn add_user(&mut self, login: String, link: String) {
        self.users.push((login, link));
    }

    pub fn render<'b, W: std::fmt::Write>(&self, buf: Buffer<W, Body<'b>>) -> Buffer<W, Body<'b>> {
        buf.node("div")
            .attr(("class", "list"))
            .attr(("attr-provider", "profiles"))
            .content(|buf| {
                self.users.iter().fold(buf, |buf, (login, link)| {
                    buf.node("a")
                        .attr(("class", "list-item"))
                        .attr(("href", link.as_str()))
                        .content(|buf| buf.text("Login as ").text(login.as_str()))
                })
            })
    }
}
