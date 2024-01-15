use url::Url;

const HEADER_ACCEPT: &str = "application/vnd.github.v3+json";
const HEADER_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

#[derive(Debug, serde::Deserialize)]
pub struct GithubProviderConfig {
    #[serde(default = "GithubProviderConfig::default_authorization_url")]
    pub authorization_url: Url,
    #[serde(default = "GithubProviderConfig::default_token_url")]
    pub token_url: Url,
    #[serde(default = "GithubProviderConfig::default_base_api_url")]
    pub base_api_url: Url,
}

impl GithubProviderConfig {
    fn default_authorization_url() -> Url {
        Url::parse("https://github.com/login/oauth/authorize")
            .expect("couldn't parse github default authorization url")
    }

    fn default_token_url() -> Url {
        Url::parse("https://github.com/login/oauth/access_token")
            .expect("couldn't parse github default token url")
    }

    fn default_base_api_url() -> Url {
        Url::parse("https://api.github.com").expect("couldn't parse github default base api url")
    }
}

#[derive(Debug)]
pub struct GithubProviderClient<'a> {
    access_token: &'a str,
    base_api_url: Url,
}

impl<'a> GithubProviderClient<'a> {
    pub fn new(access_token: &'a str, base_api_url: Url) -> Self {
        Self {
            access_token,
            base_api_url,
        }
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn fetch_user(&self) -> Result<GithubUser, String> {
        let url = format!("{}/user", self.base_api_url).replace("//", "/");
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
