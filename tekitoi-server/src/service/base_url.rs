use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct BaseUrl(Arc<String>);

impl From<String> for BaseUrl {
    fn from(value: String) -> Self {
        Self(Arc::new(value))
    }
}

impl AsRef<str> for BaseUrl {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
