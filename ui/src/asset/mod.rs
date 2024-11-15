pub const STYLE_CSS_CONTENT: &[u8] = include_bytes!("./style.css");
pub const STYLE_CSS_PATH: &str = concat!("/assets/style-", env!("CARGO_PKG_VERSION"), ".css");
