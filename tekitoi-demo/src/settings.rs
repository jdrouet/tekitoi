use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

#[derive(Debug, serde::Deserialize)]
pub struct Settings {
    #[serde(default = "Settings::default_host")]
    host: String,
    #[serde(default = "Settings::default_port")]
    port: u16,
    //
    #[serde(default = "Settings::default_redis_url")]
    redis_url: String,
    //
    #[serde(default = "Settings::default_client_id")]
    client_id: String,
    #[serde(default = "Settings::default_client_secret")]
    client_secret: String,
    #[serde(default = "Settings::default_auth_url")]
    auth_url: String,
    #[serde(default = "Settings::default_token_url")]
    token_url: String,
    #[serde(default = "Settings::default_api_url")]
    pub api_url: String,
    base_url: Option<String>,
}

impl Settings {
    fn default_host() -> String {
        "localhost".into()
    }

    fn default_port() -> u16 {
        8080
    }

    fn default_redis_url() -> String {
        "redis://localhost".into()
    }

    fn default_client_id() -> String {
        "demo-client-id".into()
    }

    fn default_client_secret() -> String {
        "demo-client-secret".into()
    }

    fn default_auth_url() -> String {
        "http://localhost:3000/authorize".into()
    }

    fn default_token_url() -> String {
        "http://localhost:3000/api/access-token".into()
    }

    fn default_api_url() -> String {
        "http://localhost:3000".into()
    }
}

impl Settings {
    pub fn build() -> Self {
        config::Config::builder()
            .add_source(config::Environment::default())
            .build()
            .expect("couldn't merge with environment")
            .try_deserialize()
            .expect("couldn't build settings")
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn base_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", self.host, self.port))
    }

    fn redirect_url(&self) -> String {
        format!("{}/api/redirect", self.base_url())
    }

    pub fn redis_client(&self) -> redis::Client {
        redis::Client::open(self.redis_url.as_str()).expect("couldn't build redis client")
    }

    pub fn oauth_client(&self) -> BasicClient {
        BasicClient::new(
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
            AuthUrl::new(self.auth_url.clone()).expect("invalid auth url"),
            Some(TokenUrl::new(self.token_url.clone()).expect("invalid token url")),
        )
        // Set the URL the user will be redirected to after the authorization process.
        .set_redirect_uri(RedirectUrl::new(self.redirect_url()).expect("invalid redirect url"))
    }
}
