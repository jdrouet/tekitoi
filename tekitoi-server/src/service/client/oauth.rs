use url::Url;

// const HEADER_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OauthProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub authorization_url: Url,
    pub token_url: Url,
    pub api_user_url: Url,
}

impl OauthProviderConfig {
    pub fn provider_client<'a>(&self, access_token: &'a str) -> OauthProviderClient<'a> {
        OauthProviderClient {
            access_token,
            api_user_url: self.api_user_url.clone(),
        }
    }
}

#[derive(Debug)]
pub struct OauthProviderClient<'a> {
    access_token: &'a str,
    api_user_url: Url,
}

impl<'a> OauthProviderClient<'a> {
    pub fn new(access_token: &'a str, api_user_url: Url) -> Self {
        Self {
            access_token,
            api_user_url,
        }
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn fetch_user(&self) -> Result<OauthUser, String> {
        tracing::debug!("fetching url {:?}", self.api_user_url);
        let response = reqwest::Client::new()
            .get(self.api_user_url.as_str())
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
pub struct OauthUser(serde_json::Value);
