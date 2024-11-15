mod helper;

#[test]
fn main() {
    helper::write(
        "/view-error.html",
        tekitoi_ui::view::error::View::new("This is an error.").with_style_path("style.css"),
    );
}
