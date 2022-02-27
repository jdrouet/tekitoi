use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use url::Url;

pub const KIND: &str = "google";

#[derive(Debug, serde::Deserialize)]
pub struct GoogleProviderSettings {
    client_id: String,
    client_secret: String,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default = "GoogleProviderSettings::default_auth_url")]
    auth_url: Url,
    #[serde(default = "GoogleProviderSettings::default_token_url")]
    token_url: Url,
    #[serde(default = "GoogleProviderSettings::default_base_api_url")]
    base_api_url: Url,
}

impl GoogleProviderSettings {
    pub fn default_auth_url() -> Url {
        Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
            .expect("unable to build default google auth url")
    }

    pub fn default_token_url() -> Url {
        Url::parse("https://oauth2.googleapis.com/token")
            .expect("unable to build default google token url")
    }

    pub fn default_base_api_url() -> Url {
        Url::parse("https://www.googleapis.com/oauth2/v1")
            .expect("unable to build default google api base url")
    }
}

impl GoogleProviderSettings {
    pub fn build(&self, base_url: &str) -> anyhow::Result<GoogleProvider> {
        tracing::trace!("build google provider base_url={:?}", base_url);
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
        Ok(GoogleProvider {
            client,
            scopes: self.scopes.clone(),
            base_api_url: self.base_api_url.clone(),
        })
    }
}

#[derive(Debug)]
pub struct GoogleProvider {
    pub client: BasicClient,
    pub scopes: Vec<String>,
    pub base_api_url: Url,
}

impl GoogleProvider {
    pub fn get_oauth_client(&self) -> &BasicClient {
        &self.client
    }

    pub fn get_oauth_scopes(&self) -> &Vec<String> {
        &self.scopes
    }

    pub fn get_api_client<'a>(&self, access_token: &'a str) -> GoogleProviderClient<'a> {
        GoogleProviderClient {
            access_token,
            base_api_url: self.base_api_url.clone(),
        }
    }
}

const HEADER_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub struct GoogleProviderClient<'a> {
    access_token: &'a str,
    base_api_url: Url,
}

impl<'a> GoogleProviderClient<'a> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn fetch_user(&self) -> Result<GoogleUser, String> {
        let url = format!("{}/userinfo", self.base_api_url);
        tracing::debug!("fetching url {:?}", url);
        let response = reqwest::Client::new()
            .get(url)
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
pub struct GoogleUser {
    pub id: String,
    pub email: Option<String>,
    pub verified_email: bool,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub locale: Option<String>,
    pub picture: Option<String>,
}
