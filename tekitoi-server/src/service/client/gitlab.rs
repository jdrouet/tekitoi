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
    #[serde(default = "GitlabProviderSettings::default_base_api_url")]
    base_api_url: Url,
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

    pub fn default_base_api_url() -> Url {
        Url::parse("https://gitlab.com").expect("unable to build default gitlab api base url")
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
            base_api_url: self.base_api_url.clone(),
        })
    }
}

#[derive(Debug)]
pub struct GitlabProvider {
    pub client: BasicClient,
    pub scopes: Vec<String>,
    pub base_api_url: Url,
}

impl GitlabProvider {
    pub fn get_oauth_client(&self) -> &BasicClient {
        &self.client
    }

    pub fn get_oauth_scopes(&self) -> &Vec<String> {
        &self.scopes
    }

    pub fn get_api_client<'a>(&self, access_token: &'a str) -> GitlabProviderClient<'a> {
        GitlabProviderClient {
            access_token,
            base_api_url: self.base_api_url.clone(),
        }
    }
}

#[derive(Debug)]
pub struct GitlabProviderClient<'a> {
    access_token: &'a str,
    base_api_url: Url,
}

impl<'a> GitlabProviderClient<'a> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn fetch_user(&self) -> Result<GitlabUser, String> {
        let url = format!("{}api/v4/user", self.base_api_url);
        let response = reqwest::Client::new()
            .get(url)
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
pub struct GitlabUser {
    pub id: u64,
    pub username: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}
