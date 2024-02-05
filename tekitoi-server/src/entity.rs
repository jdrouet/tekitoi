use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AuthorizationState(String);

impl AsRef<str> for AuthorizationState {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RedirectUri(Url);

impl AsRef<str> for RedirectUri {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<Url> for RedirectUri {
    fn as_ref(&self) -> &Url {
        &self.0
    }
}
