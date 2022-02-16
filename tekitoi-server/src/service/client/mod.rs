pub mod github;
pub mod gitlab;

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

    pub fn get_client(&self, client_id: &str) -> Option<&Client> {
        self.0.get(client_id)
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
    gitlab: Option<gitlab::GitlabProviderSettings>,
}

impl ProviderManagerSettings {
    pub fn build(&self, base_url: &str) -> anyhow::Result<ProviderManager> {
        Ok(ProviderManager {
            github: self
                .github
                .as_ref()
                .map(|gh| gh.build(base_url))
                .transpose()?,
            gitlab: self
                .gitlab
                .as_ref()
                .map(|gl| gl.build(base_url))
                .transpose()?,
        })
    }
}

#[derive(Debug, Default)]
pub struct ProviderManager {
    github: Option<github::GithubProvider>,
    gitlab: Option<gitlab::GitlabProvider>,
}

impl ProviderManager {
    pub fn get_provider(&self, kind: &str) -> Option<&oauth2::basic::BasicClient> {
        match kind {
            github::KIND => self.github.as_ref().map(|gh| &gh.client),
            gitlab::KIND => self.gitlab.as_ref().map(|gl| &gl.client),
            _ => None,
        }
    }

    pub fn with_scopes<'a>(
        &self,
        kind: &str,
        req: oauth2::AuthorizationRequest<'a>,
    ) -> oauth2::AuthorizationRequest<'a> {
        if let Some(scopes) = self.get_scopes(kind) {
            scopes.iter().fold(req, |r, scope| {
                r.add_scope(oauth2::Scope::new(scope.clone()))
            })
        } else {
            req
        }
    }

    pub fn get_scopes(&self, kind: &str) -> Option<&Vec<String>> {
        match kind {
            github::KIND => self.github.as_ref().map(|gh| &gh.scopes),
            gitlab::KIND => self.gitlab.as_ref().map(|gl| &gl.scopes),
            _ => None,
        }
    }
}
