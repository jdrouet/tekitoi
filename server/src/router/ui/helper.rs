use std::borrow::Cow;
use std::collections::HashMap;

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
