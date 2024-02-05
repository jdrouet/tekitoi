use url::Url;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GitlabProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default = "GitlabProviderConfig::default_authorization_url")]
    pub authorization_url: Url,
    #[serde(default = "GitlabProviderConfig::default_token_url")]
    pub token_url: Url,
    #[serde(default = "GitlabProviderConfig::default_base_api_url")]
    pub base_api_url: Url,
}

impl GitlabProviderConfig {
    fn default_authorization_url() -> Url {
        Url::parse("http://gitlab.com/oauth/authorize")
            .expect("couldn't parse gitlab default authorization url")
    }

    fn default_token_url() -> Url {
        Url::parse("https://gitlab.com/oauth/token")
            .expect("couldn't parse gitlab default token url")
    }

    fn default_base_api_url() -> Url {
        Url::parse("https://gitlab.com").expect("couldn't parse gitlab default base api url")
    }

    pub fn provider_client<'a>(&self, access_token: &'a str) -> GitlabProviderClient<'a> {
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
    pub fn new(access_token: &'a str, base_api_url: Url) -> Self {
        Self {
            access_token,
            base_api_url,
        }
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn fetch_user(&self) -> Result<GitlabUser, String> {
        let url = format!("{}/api/v4/user", self.base_api_url);
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
