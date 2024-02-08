use url::Url;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct OauthProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub authorization_url: Url,
    pub token_url: Url,
    pub api_user_url: Url,
}

impl OauthProviderConfig {
    pub(crate) fn provider_client(self, access_token: String) -> Box<dyn super::ProviderClient> {
        Box::new(OauthProviderClient {
            access_token,
            api_user_url: self.api_user_url,
        })
    }
}

#[derive(Debug)]
pub(crate) struct OauthProviderClient {
    access_token: String,
    api_user_url: Url,
}

impl OauthProviderClient {
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

#[axum::async_trait]
impl super::ProviderClient for OauthProviderClient {
    #[tracing::instrument(level = "debug", skip_all)]
    async fn fetch_user(&self) -> Result<super::ProviderUser, String> {
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
            response
                .json()
                .await
                .map(super::ProviderUser::Oauth)
                .map_err(|err| err.to_string())
        } else {
            let error = response.text().await.map_err(|err| err.to_string())?;
            Err(error)
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct OauthUser(serde_json::Value);
