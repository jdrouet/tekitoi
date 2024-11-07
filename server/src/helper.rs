use std::str::FromStr;

use anyhow::Context;

#[inline(always)]
pub(crate) fn from_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("getting environment variable {name:?}"))
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
