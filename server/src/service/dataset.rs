use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::Context;
use uuid::Uuid;

use crate::entity::user::Entity as UserEntity;
use crate::helper::parse_env_or;

#[cfg(test)]
pub(crate) const CLIENT_ID: Uuid = Uuid::from_u128(0x00010000000000000000000000000000u128);
#[cfg(test)]
pub(crate) const REDIRECT_URI: &str = "http://service/redirect";
#[cfg(test)]
pub(crate) const ALICE_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000000u128);
#[cfg(test)]
pub(crate) const BOB_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000001u128);

pub(crate) struct Config {
    path: PathBuf,
}

impl Config {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            path: parse_env_or("CONFIG_PATH", PathBuf::from("./config.json"))?,
        })
    }

    pub(crate) async fn synchronize(
        &self,
        database: &crate::service::database::Pool,
    ) -> anyhow::Result<()> {
        let root = RootConfig::from_path(&self.path)?;
        root.synchronize(database).await
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct RootConfig {
    applications: Vec<ApplicationConfig>,
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

impl RootConfig {
    pub(crate) async fn synchronize(
        &self,
        database: &crate::service::database::Pool,
    ) -> anyhow::Result<()> {
        tracing::debug!("executing synchro");
        let mut tx = database.as_ref().begin().await?;
        for app in self.applications.iter() {
            crate::entity::application::Upsert::new(
                app.client_id,
                &app.client_secrets,
                &app.redirect_uri,
            )
            .execute(&mut *tx)
            .await?;

            for user in app.users.iter() {
                crate::entity::user::Upsert::new(user.id, app.client_id, &user.login, &user.email)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
impl RootConfig {
    pub(crate) fn test() -> Self {
        RootConfig {
            applications: vec![ApplicationConfig {
                client_id: CLIENT_ID,
                redirect_uri: REDIRECT_URI.into(),
                client_secrets: HashSet::from_iter([String::from("secret")]),
                users: vec![
                    UserEntity {
                        id: ALICE_ID,
                        login: "alice".into(),
                        email: "alice@gmail.com".into(),
                    },
                    UserEntity {
                        id: BOB_ID,
                        login: "bob".into(),
                        email: "bob@gmail.com".into(),
                    },
                ],
            }],
        }
    }
}

#[derive(serde::Deserialize)]
struct ApplicationConfig {
    client_id: Uuid,
    redirect_uri: String,
    client_secrets: HashSet<String>,
    users: Vec<UserEntity>,
}
