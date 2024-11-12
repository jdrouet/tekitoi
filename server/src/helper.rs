use std::{borrow::Cow, str::FromStr};

use anyhow::Context;

#[inline(always)]
pub(crate) fn from_env_or(name: &str, default_value: &'static str) -> Cow<'static, str> {
    std::env::var(name)
        .ok()
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(default_value))
}

pub(crate) fn parse_env_or<V>(name: &str, default_value: V) -> anyhow::Result<V>
where
    V: FromStr,
    <V as FromStr>::Err: Send + Sync + 'static,
    <V as FromStr>::Err: std::error::Error,
    anyhow::Error: From<<V as FromStr>::Err>,
{
    match std::env::var(name) {
        Ok(value) => {
            let parsed = value
                .parse()
                .with_context(|| format!("parsing {name}={value:?}"))?;
            Ok(parsed)
        }
        Err(_) => Ok(default_value),
    }
}

pub(crate) fn generate_token(length: usize) -> String {
    use rand::distributions::{Alphanumeric, Distribution};
    use rand::thread_rng;

    let mut rng = thread_rng();
    Alphanumeric
        .sample_iter(&mut rng)
        .take(length)
        .map(char::from)
        .collect()
}
