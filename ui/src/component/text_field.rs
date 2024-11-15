use another_html_builder::{Body, Buffer};

pub(crate) struct Component {
    pub rtype: &'static str,
    pub id: &'static str,
    pub name: &'static str,
    pub label: &'static str,
    pub placeholder: &'static str,
    pub required: bool,
}

impl Component {
    fn render_label<'a, W: std::fmt::Write>(
        &self,
        buf: Buffer<W, Body<'a>>,
    ) -> Buffer<W, Body<'a>> {
        buf.node("label")
            .attr(("for", self.id))
            .content(|buf| buf.text(self.label))
    }

    fn render_input<'a, W: std::fmt::Write>(
        &self,
        buf: Buffer<W, Body<'a>>,
    ) -> Buffer<W, Body<'a>> {
        let buf = buf
            .node("input")
            .attr(("id", self.id))
            .attr(("type", self.rtype))
            .attr(("name", self.name))
            .attr(("placeholder", self.placeholder));
        let buf = if self.required {
            buf.attr(("required", "required"))
        } else {
            buf
        };
        buf.close()
    }

    pub(crate) fn render<'a, W: std::fmt::Write>(
        &self,
        buf: Buffer<W, Body<'a>>,
    ) -> Buffer<W, Body<'a>> {
        buf.node("div")
            .attr(("class", "text-input"))
            .content(|buf| {
                let buf = self.render_label(buf);
                let buf = self.render_input(buf);
                buf
            })
    }
}
