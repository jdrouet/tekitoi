use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Context;
use uuid::Uuid;

mod credentials;
mod profiles;

#[cfg(test)]
pub(crate) const CLIENT_ID: Uuid = Uuid::from_u128(0x00010000000000000000000000000000u128);
#[cfg(test)]
pub(crate) const CLIENT_SECRET: &str = "secret";
#[cfg(test)]
pub(crate) const REDIRECT_URI: &str = "http://service/redirect";
#[cfg(test)]
pub(crate) const ALICE_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000000u128);
#[cfg(test)]
pub(crate) const BOB_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000001u128);

pub(crate) struct Config {
    path: Option<PathBuf>,
}

impl Config {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            path: std::env::var("CONFIG_PATH").ok().map(PathBuf::from),
        })
    }

    pub(crate) async fn synchronize(
        &self,
        database: &crate::service::database::Pool,
    ) -> anyhow::Result<()> {
        if let Some(ref path) = self.path {
            tracing::debug!("synchronizing with provided configuration");
            let root = RootConfig::from_path(path)?;
            root.synchronize(database).await
        } else {
            tracing::debug!("no configuration path provided, skipping...");
            Ok(())
        }
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
            let created = crate::entity::application::Upsert::new(
                app.client_id,
                &app.client_secrets,
                &app.redirect_uri,
            )
            .execute(&mut *tx)
            .await?;

            for provider in app.providers.iter() {
                tx = provider.synchronize(tx, &created).await?;
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
                client_secrets: HashSet::from_iter([CLIENT_SECRET.into()]),
                providers: vec![Provider::Profiles(profiles::Config::test())],
            }],
        }
    }
}

#[derive(serde::Deserialize)]
struct ApplicationConfig {
    client_id: Uuid,
    redirect_uri: String,
    client_secrets: HashSet<String>,
    providers: Vec<Provider>,
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum Provider {
    Credentials(credentials::Config),
    Profiles(profiles::Config),
}

impl Provider {
    pub(super) async fn synchronize<'c>(
        &self,
        tx: sqlx::Transaction<'c, sqlx::Sqlite>,
        app: &crate::entity::application::Entity,
    ) -> anyhow::Result<sqlx::Transaction<'c, sqlx::Sqlite>> {
        match self {
            Self::Credentials(inner) => inner.synchronize(tx, app).await,
            Self::Profiles(inner) => inner.synchronize(tx, app).await,
        }
    }
}
