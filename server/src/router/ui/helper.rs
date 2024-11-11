use std::{borrow::Cow, collections::HashMap, fmt::Display};

#[inline(always)]
pub(super) fn doctype(f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("<!DOCTYPE html>")
}

pub(super) fn redirection<T: Display>(target: T) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="1; url='{}'" /></head><body><p>You will be redirected soon...</p></body></html>"#,
        target
    )
}

pub(super) fn encode_params<'a>(
    values: impl Iterator<Item = (&'a str, &'a str)>,
) -> Option<String> {
    let params = HashMap::<&str, &str>::from_iter(values);
    if params.is_empty() {
        None
    } else {
        serde_urlencoded::to_string(&params).ok()
    }
}

pub(super) fn encode_url<'a>(
    path: &'a str,
    params: impl Iterator<Item = (&'a str, &'a str)>,
) -> Cow<'a, str> {
    match encode_params(params) {
        Some(values) => Cow::Owned(format!("{path}?{values}")),
        None => Cow::Borrowed(path),
    }
}
