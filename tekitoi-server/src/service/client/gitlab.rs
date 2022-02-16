use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use url::Url;

pub const KIND: &str = "gitlab";

#[derive(Debug, serde::Deserialize)]
pub struct GitlabProviderSettings {
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
    #[serde(default = "GitlabProviderSettings::default_auth_url")]
    auth_url: Url,
    #[serde(default = "GitlabProviderSettings::default_token_url")]
    token_url: Url,
}

impl GitlabProviderSettings {
    pub fn default_auth_url() -> Url {
        Url::parse("https://gitlab.com/oauth/authorize")
            .expect("unable to build default gitlab auth url")
    }

    pub fn default_token_url() -> Url {
        Url::parse("https://gitlab.com/oauth/token")
            .expect("unable to build default gitlab token url")
    }
}

impl GitlabProviderSettings {
    pub fn build(&self, base_url: &str) -> anyhow::Result<GitlabProvider> {
        tracing::trace!("build gitlab provider base_url={:?}", base_url);
        let client = BasicClient::new(
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
            AuthUrl::from_url(self.auth_url.clone()),
            Some(TokenUrl::from_url(self.token_url.clone())),
        )
        .set_redirect_uri(RedirectUrl::new(format!(
            "{}/api/redirect/{}",
            base_url, KIND
        ))?);
        Ok(GitlabProvider {
            client,
            scopes: self.scopes.clone(),
        })
    }
}

#[derive(Debug)]
pub struct GitlabProvider {
    pub client: BasicClient,
    pub scopes: Vec<String>,
}
