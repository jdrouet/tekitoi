use another_html_builder::{Body, Buffer};

pub(crate) fn render<'a, W: std::fmt::Write>(
    buf: Buffer<W, Body<'a>>,
    style_path: Option<&'static str>,
) -> Buffer<W, Body<'a>> {
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
            .attr(("href", style_path.unwrap_or(crate::asset::STYLE_CSS_PATH)))
            .close()
    })
}
