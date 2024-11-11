use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use uuid::Uuid;

use crate::entity::user::Entity as UserEntity;
use crate::helper::parse_env_or;

#[cfg(test)]
pub(crate) const APP_ID: &str = "client-id";
#[cfg(test)]
pub(crate) const REDIRECT_URI: &str = "http://service/redirect";
#[cfg(test)]
pub(crate) const ALICE_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000000u128);
#[cfg(test)]
pub(crate) const BOB_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000001u128);

#[derive(serde::Deserialize)]
struct RootConfig {
    applications: Vec<ApplicationConfig>,
}

#[derive(serde::Deserialize)]
struct ApplicationConfig {
    client_id: String,
    redirect_uri: String,
    client_secrets: HashSet<String>,
    users: Vec<UserEntity>,
}

impl ApplicationConfig {
    fn build(self) -> (String, ApplicationClient) {
        (
            self.client_id,
            ApplicationClient {
                redirect_uri: self.redirect_uri,
                client_secrets: self.client_secrets,
                users: self.users,
            },
        )
    }
}

impl RootConfig {
    fn from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(path)
            .context("opening configuration file")?;
        serde_json::from_reader(file).context("reading configuration file")
    }
}

pub(crate) struct Config {
    path: PathBuf,
}

impl Config {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            path: parse_env_or("CONFIG_PATH", PathBuf::from("./config.json"))?,
        })
    }

    pub(crate) fn build(self) -> anyhow::Result<Client> {
        let root = RootConfig::from_path(self.path)?;
        let entries = HashMap::from_iter(root.applications.into_iter().map(|app| app.build()));
        Ok(Client(Arc::new(entries)))
    }
}

#[derive(Debug)]
pub(crate) struct ApplicationClient {
    client_secrets: HashSet<String>,
    redirect_uri: String,
    users: Vec<UserEntity>,
}

impl ApplicationClient {
    pub(crate) fn check_redirect_uri(&self, redirect_uri: &str) -> bool {
        self.redirect_uri.eq(redirect_uri)
    }

    pub(crate) fn check_client_secret(&self, secret: &str) -> bool {
        self.client_secrets.contains(secret)
    }

    pub(crate) fn users(&self) -> &[UserEntity] {
        &self.users
    }

    pub(crate) fn user(&self, user_id: Uuid) -> Option<&UserEntity> {
        self.users.iter().find(|u| u.id == user_id)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Client(Arc<HashMap<String, ApplicationClient>>);

impl Client {
    pub fn find(&self, client_id: &str) -> Option<&ApplicationClient> {
        self.0.get(client_id)
    }
}

#[cfg(test)]
impl Client {
    pub(crate) fn test() -> Self {
        Self(Arc::new(HashMap::from_iter([(
            APP_ID.to_string(),
            ApplicationClient {
                client_secrets: HashSet::from_iter([
                    "first-secret".to_string(),
                    "second-secret".to_string(),
                ]),
                redirect_uri: "http://service/redirect".into(),
                users: vec![
                    UserEntity {
                        id: ALICE_ID,
                        login: "alice".into(),
                        email: "alice@example.com".into(),
                    },
                    UserEntity {
                        id: BOB_ID,
                        login: "bob".into(),
                        email: "bob@example.com".into(),
                    },
                ],
            },
        )])))
    }
}
