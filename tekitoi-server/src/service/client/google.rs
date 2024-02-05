use url::Url;

const HEADER_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GoogleProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default = "GoogleProviderConfig::default_authorization_url")]
    pub authorization_url: Url,
    #[serde(default = "GoogleProviderConfig::default_token_url")]
    pub token_url: Url,
    #[serde(default = "GoogleProviderConfig::default_base_api_url")]
    pub base_api_url: Url,
}

impl GoogleProviderConfig {
    fn default_authorization_url() -> Url {
        Url::parse("https://accounts.google.com/o/oauth2/auth")
            .expect("couldn't parse google default authorization url")
    }

    fn default_token_url() -> Url {
        Url::parse("https://oauth2.googleapis.com/token")
            .expect("couldn't parse google default token url")
    }

    fn default_base_api_url() -> Url {
        Url::parse("https://www.googleapis.com/oauth2/v1")
            .expect("couldn't parse google default base api url")
    }

    pub fn provider_client<'a>(&self, access_token: &'a str) -> GoogleProviderClient<'a> {
        GoogleProviderClient {
            access_token,
            base_api_url: self.base_api_url.clone(),
        }
    }
}

#[derive(Debug)]
pub struct GoogleProviderClient<'a> {
    access_token: &'a str,
    base_api_url: Url,
}

impl<'a> GoogleProviderClient<'a> {
    pub fn new(access_token: &'a str, base_api_url: Url) -> Self {
        Self {
            access_token,
            base_api_url,
        }
    }

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
