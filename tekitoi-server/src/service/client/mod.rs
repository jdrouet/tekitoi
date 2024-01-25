pub mod github;
pub mod gitlab;
pub mod google;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use url::Url;

#[derive(Debug, Default, serde::Deserialize)]
pub struct ClientManagerSettings(HashMap<String, ClientSettings>);

impl ClientManagerSettings {
    pub fn build(&self, base_url: &str) -> anyhow::Result<ClientManager> {
        Ok(ClientManager(Arc::new(
            self.0
                .iter()
                .map(|(name, item)| item.build(name.as_str(), base_url))
                .collect::<anyhow::Result<HashMap<_, _>>>()?,
        )))
    }
}

#[derive(Clone)]
pub struct ClientManager(Arc<HashMap<String, Client>>);

impl ClientManager {
    pub fn get(&self, client_id: &str) -> Result<&Client, &'static str> {
        self.0.get(client_id).ok_or("Client not found.")
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ClientSettings {
    pub client_id: String,
    pub client_secrets: HashSet<String>,
    pub redirect_uri: Url,
    #[serde(default)]
    pub providers: ProviderManagerSettings,
}

impl ClientSettings {
    pub fn build(&self, name: &str, base_url: &str) -> anyhow::Result<(String, Client)> {
        let providers = self.providers.build(base_url)?;
        Ok((
            self.client_id.clone(),
            Client {
                name: name.to_string(),
                client_secrets: self.client_secrets.clone(),
                redirect_uri: self.redirect_uri.clone(),
                providers,
            },
        ))
    }
}

pub struct Client {
    pub name: String,
    pub client_secrets: HashSet<String>,
    pub redirect_uri: Url,
    pub providers: ProviderManager,
}

impl Client {
    pub fn check_redirect_uri(&self, url: &Url) -> Result<(), &'static str> {
        if &self.redirect_uri == url {
            Ok(())
        } else {
            tracing::trace!(
                "invalid redirect uri, expected {:?}, got {:?}",
                self.redirect_uri.to_string(),
                url.to_string()
            );
            Err("Invalid redirect uri.")
        }
    }
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct ProviderManagerSettings {
    github: Option<github::GithubProviderSettings>,
    gitlab: Option<gitlab::GitlabProviderSettings>,
    google: Option<google::GoogleProviderSettings>,
}

impl ProviderManagerSettings {
    pub fn build(&self, base_url: &str) -> anyhow::Result<ProviderManager> {
        let mut res = HashMap::<&'static str, Provider>::new();
        if let Some(item) = self.github.as_ref() {
            let provider = item.build(base_url)?;
            res.insert(github::KIND, provider.into());
        }
        if let Some(item) = self.gitlab.as_ref() {
            let provider = item.build(base_url)?;
            res.insert(gitlab::KIND, provider.into());
        }
        if let Some(item) = self.google.as_ref() {
            let provider = item.build(base_url)?;
            res.insert(google::KIND, provider.into());
        }
        Ok(ProviderManager(res))
    }
}

#[derive(Debug, Default)]
pub struct ProviderManager(HashMap<&'static str, Provider>);

impl ProviderManager {
    pub fn get(&self, kind: &str) -> Option<&Provider> {
        self.0.get(kind)
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.0.keys().copied().collect()
    }
}

#[derive(Debug)]
pub enum Provider {
    Github(github::GithubProvider),
    Gitlab(gitlab::GitlabProvider),
    Google(google::GoogleProvider),
}

impl From<github::GithubProvider> for Provider {
    fn from(value: github::GithubProvider) -> Self {
        Self::Github(value)
    }
}

impl From<gitlab::GitlabProvider> for Provider {
    fn from(value: gitlab::GitlabProvider) -> Self {
        Self::Gitlab(value)
    }
}

impl From<google::GoogleProvider> for Provider {
    fn from(value: google::GoogleProvider) -> Self {
        Self::Google(value)
    }
}

impl Provider {
    pub fn get_oauth_client(&self) -> &oauth2::basic::BasicClient {
        match self {
            Self::Github(item) => item.get_oauth_client(),
            Self::Gitlab(item) => item.get_oauth_client(),
            Self::Google(item) => item.get_oauth_client(),
        }
    }

    pub fn with_oauth_scopes<'a>(
        &self,
        req: oauth2::AuthorizationRequest<'a>,
    ) -> oauth2::AuthorizationRequest<'a> {
        self.get_oauth_scopes().iter().fold(req, |r, scope| {
            r.add_scope(oauth2::Scope::new(scope.clone()))
        })
    }

    pub fn get_oauth_scopes(&self) -> &Vec<String> {
        match self {
            Self::Github(item) => item.get_oauth_scopes(),
            Self::Gitlab(item) => item.get_oauth_scopes(),
            Self::Google(item) => item.get_oauth_scopes(),
        }
    }

    pub fn get_api_client<'a>(&self, access_token: &'a str) -> ProviderClient<'a> {
        match self {
            Self::Github(item) => item.get_api_client(access_token).into(),
            Self::Gitlab(item) => item.get_api_client(access_token).into(),
            Self::Google(item) => item.get_api_client(access_token).into(),
        }
    }
}

#[derive(Debug)]
pub enum ProviderClient<'a> {
    Github(github::GithubProviderClient<'a>),
    Gitlab(gitlab::GitlabProviderClient<'a>),
    Google(google::GoogleProviderClient<'a>),
}

impl<'a> From<github::GithubProviderClient<'a>> for ProviderClient<'a> {
    fn from(value: github::GithubProviderClient<'a>) -> Self {
        Self::Github(value)
    }
}

impl<'a> From<gitlab::GitlabProviderClient<'a>> for ProviderClient<'a> {
    fn from(value: gitlab::GitlabProviderClient<'a>) -> Self {
        Self::Gitlab(value)
    }
}

impl<'a> From<google::GoogleProviderClient<'a>> for ProviderClient<'a> {
    fn from(value: google::GoogleProviderClient<'a>) -> Self {
        Self::Google(value)
    }
}

impl<'a> ProviderClient<'a> {
    pub async fn fetch_user(&self) -> Result<ProviderUser, String> {
        match self {
            Self::Github(client) => client.fetch_user().await.map(Into::into),
            Self::Gitlab(client) => client.fetch_user().await.map(Into::into),
            Self::Google(client) => client.fetch_user().await.map(Into::into),
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "provider", rename_all = "kebab-case")]
pub enum ProviderUser {
    Github(github::GithubUser),
    Gitlab(gitlab::GitlabUser),
    Google(google::GoogleUser),
}

impl From<github::GithubUser> for ProviderUser {
    fn from(value: github::GithubUser) -> Self {
        Self::Github(value)
    }
}

impl From<gitlab::GitlabUser> for ProviderUser {
    fn from(value: gitlab::GitlabUser) -> Self {
        Self::Gitlab(value)
    }
}

impl From<google::GoogleUser> for ProviderUser {
    fn from(value: google::GoogleUser) -> Self {
        Self::Google(value)
    }
}
