use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use url::Url;

pub const KIND: &str = "github";

#[derive(Debug, serde::Deserialize)]
pub struct GithubProviderSettings {
    client_id: String,
    client_secret: String,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default = "GithubProviderSettings::default_auth_url")]
    auth_url: Url,
    #[serde(default = "GithubProviderSettings::default_token_url")]
    token_url: Url,
    #[serde(default = "GithubProviderSettings::default_base_api_url")]
    base_api_url: Url,
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

    pub fn default_base_api_url() -> Url {
        Url::parse("https://api.github.com").expect("unable to build default github api base url")
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
        Ok(GithubProvider {
            client,
            scopes: self.scopes.clone(),
            base_api_url: self.base_api_url.clone(),
        })
    }
}

#[derive(Debug)]
pub struct GithubProvider {
    pub client: BasicClient,
    pub scopes: Vec<String>,
    pub base_api_url: Url,
}

impl GithubProvider {
    pub fn get_oauth_client(&self) -> &BasicClient {
        &self.client
    }

    pub fn get_oauth_scopes(&self) -> &Vec<String> {
        &self.scopes
    }

    pub fn get_api_client<'a>(&self, access_token: &'a str) -> GithubProviderClient<'a> {
        GithubProviderClient {
            access_token,
            base_api_url: self.base_api_url.clone(),
        }
    }
}

const HEADER_ACCEPT: &str = "application/vnd.github.v3+json";
const HEADER_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub struct GithubProviderClient<'a> {
    access_token: &'a str,
    base_api_url: Url,
}

impl<'a> GithubProviderClient<'a> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn fetch_user(&self) -> Result<GithubUser, String> {
        let url = format!("{}user", self.base_api_url);
        tracing::debug!("fetching url {:?}", url);
        let response = reqwest::Client::new()
            .get(url)
            .header(reqwest::header::ACCEPT, HEADER_ACCEPT)
            .header(reqwest::header::USER_AGENT, HEADER_USER_AGENT)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.access_token),
            )
            .send()
            .await
            .map_err(|err| err.to_string())?;
        tracing::debug!("received response {:?}", response.status());
        if response.status().is_success() {
            response.json().await.map_err(|err| err.to_string())
        } else {
            let error = response.text().await.map_err(|err| err.to_string())?;
            Err(error)
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GithubUser {
    pub id: u64,
    pub login: Option<String>,
    pub node_id: Option<String>,
    pub avatar_url: Option<String>,
    pub html_url: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}
