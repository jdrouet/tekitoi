use std::borrow::Cow;

use axum::response::Redirect;
use url::Url;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct AuthorizationState(String);

impl AsRef<str> for AuthorizationState {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AuthorizationState {
    pub(crate) fn inner(self) -> String {
        self.0
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct RedirectUri(Url);

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

impl RedirectUri {
    pub fn inner(self) -> Url {
        self.0
    }
}

const REDIRECT_URI_MISMATCH: &str = "redirect_uri_mismatch";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AuthorizationError {
    error: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_description: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_uri: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<String>,
}

impl AuthorizationError {
    pub(crate) fn create_redirect_uri_mismatch() -> Self {
        Self {
            error: REDIRECT_URI_MISMATCH.into(),
            error_description: Some(
                "The redirect_uri MUST match the registered callback URL for this application."
                    .into(),
            ),
            error_uri: None,
            state: None,
        }
    }

    pub(crate) fn with_state(mut self, state: String) -> Self {
        self.state = Some(state);
        self
    }

    pub(crate) fn error_description(&self) -> Option<&str> {
        self.error_description.as_deref()
    }

    pub(crate) fn state(&self) -> Option<&str> {
        self.state.as_deref()
    }

    pub(crate) fn as_url_params(&self) -> String {
        serde_url_params::to_string(&self).expect("couldn't url encode error")
    }

    pub(crate) fn as_redirect(&self, mut redirect_url: Url) -> Redirect {
        let params = self.as_url_params();
        redirect_url.set_query(Some(&params));
        Redirect::temporary(redirect_url.as_str())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AuthorizationRedirect {
    code: String,
    state: Option<String>,
}

impl AuthorizationRedirect {
    pub(crate) fn new(code: String, state: String) -> Self {
        Self {
            code,
            state: Some(state),
        }
    }

    pub(crate) fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn state(&self) -> Option<&str> {
        self.state.as_deref()
    }

    pub(crate) fn as_url_params(&self) -> String {
        serde_url_params::to_string(&self).expect("couldn't url encode redirect")
    }

    pub(crate) fn as_redirect(&self, mut redirect_url: Url) -> Redirect {
        let params = self.as_url_params();
        redirect_url.set_query(Some(&params));
        Redirect::temporary(redirect_url.as_str())
    }
}
