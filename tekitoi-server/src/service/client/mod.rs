pub mod github;

use std::collections::{HashMap, HashSet};
use url::Url;

#[derive(Debug, Default, serde::Deserialize)]
pub struct ClientManagerSettings(HashMap<String, ClientSettings>);

impl ClientManagerSettings {
    pub fn build(&self, base_url: &str) -> anyhow::Result<ClientManager> {
        Ok(ClientManager(
            self.0
                .iter()
                .map(|(name, item)| item.build(name.as_str(), base_url))
                .collect::<anyhow::Result<HashMap<_, _>>>()?,
        ))
    }
}

pub struct ClientManager(HashMap<String, Client>);

impl<'a> ClientManager {
    pub fn validate(&self, client_id: &str, redirect_uri: &Url) -> Result<(), &'static str> {
        if let Some(client) = self.0.get(client_id) {
            if &client.redirect_uri == redirect_uri {
                Ok(())
            } else {
                tracing::trace!(
                    "invalid redirect uri, expected {:?}, got {:?}",
                    client.redirect_uri.to_string(),
                    redirect_uri.to_string()
                );
                Err("Invalid redirect uri.")
            }
        } else {
            Err("Client not found.")
        }
    }

    pub fn get_oauth_client(
        &self,
        client_id: &str,
        provider: &str,
    ) -> Option<&oauth2::basic::BasicClient> {
        self.0
            .get(client_id)
            .and_then(|client| client.providers.get_client(provider))
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

#[derive(Debug, Default, serde::Deserialize)]
pub struct ProviderManagerSettings {
    github: Option<github::GithubProviderSettings>,
}

impl ProviderManagerSettings {
    pub fn build(&self, base_url: &str) -> anyhow::Result<ProviderManager> {
        Ok(ProviderManager {
            github: self
                .github
                .as_ref()
                .map(|gh| gh.build(base_url))
                .transpose()?,
        })
    }
}

#[derive(Debug, Default)]
pub struct ProviderManager {
    github: Option<github::GithubProvider>,
}

impl ProviderManager {
    pub fn get_client(&self, kind: &str) -> Option<&oauth2::basic::BasicClient> {
        match kind {
            github::KIND => self.github.as_ref().map(|gh| &gh.client),
            _ => None,
        }
    }
}
