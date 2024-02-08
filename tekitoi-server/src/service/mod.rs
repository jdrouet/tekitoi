use std::sync::Arc;

pub mod client;
pub mod database;

#[derive(Clone, Debug)]
pub(crate) struct BaseUrl(Arc<String>);

impl From<String> for BaseUrl {
    fn from(inner: String) -> Self {
        Self(Arc::new(inner))
    }
}

impl AsRef<str> for BaseUrl {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
