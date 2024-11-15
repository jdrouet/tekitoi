pub fn write<V: tekitoi_ui::view::View>(filename: &str, view: V) {
    let target = concat!(env!("CARGO_TARGET_TMPDIR"), "/style.css");
    std::fs::write(target, tekitoi_ui::asset::STYLE_CSS_CONTENT).unwrap();

    let output = view.render();
    let target = format!("{}/{filename}", env!("CARGO_TARGET_TMPDIR"));
    std::fs::write(target, output).unwrap();
}
