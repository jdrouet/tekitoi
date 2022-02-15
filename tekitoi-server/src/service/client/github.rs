use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use url::Url;

pub const KIND: &str = "github";

#[derive(Debug, serde::Deserialize)]
pub struct GithubProviderSettings {
    client_id: String,
    client_secret: String,
    #[serde(default = "GithubProviderSettings::default_auth_url")]
    auth_url: Url,
    #[serde(default = "GithubProviderSettings::default_token_url")]
    token_url: Url,
}

impl GithubProviderSettings {
    pub fn default_auth_url() -> Url {
        Url::parse("https://github.com/login/oauth/authorize")
            .expect("unable to build default github auth url")
    }

    pub fn default_token_url() -> Url {
        Url::parse("https://github.com/login/oauth/access_token")
            .expect("unable to build default github token url")
    }
}

impl GithubProviderSettings {
    pub fn build(&self, base_url: &str) -> anyhow::Result<GithubProvider> {
        tracing::trace!("build github provider base_url={:?}", base_url);
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
        Ok(GithubProvider { client })
    }
}

#[derive(Debug)]
pub struct GithubProvider {
    pub client: BasicClient,
}
