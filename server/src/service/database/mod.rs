use std::borrow::Cow;

use anyhow::Context;
use sqlx::migrate::Migrator;
use sqlx::Executor;

use crate::helper::from_env_or;

pub(crate) struct Config {
    url: Cow<'static, str>,
}

#[cfg(test)]
impl Default for Config {
    fn default() -> Self {
        Self {
            url: Cow::Borrowed(":memory:"),
        }
    }
}

impl Config {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            url: from_env_or("DATABASE_URL", ":memory:"),
        })
    }

    pub(crate) async fn build(self) -> anyhow::Result<Pool> {
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(self.url.as_ref());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            // we need at least 1 connection, otherwise it looses the data when using in memory db
            .min_connections(1)
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(opts)
            .await
            .context("building connection pool")?;
        Ok(Pool(pool))
    }
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");

#[derive(Clone, Debug)]
pub struct Pool(sqlx::sqlite::SqlitePool);

impl AsRef<sqlx::sqlite::SqlitePool> for Pool {
    fn as_ref(&self) -> &sqlx::sqlite::SqlitePool {
        &self.0
    }
}

impl Pool {
    pub async fn ping(&self) -> sqlx::Result<()> {
        self.0.execute("select 1").await?;
        Ok(())
    }

    pub async fn upgrade(&self) -> Result<(), sqlx::migrate::MigrateError> {
        tracing::debug!("executing migrations");
        MIGRATOR.run(self.as_ref()).await
    }
}
