use std::str::FromStr;

use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions};

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: Self::default_url(),
        }
    }
}

impl Config {
    pub fn default_url() -> String {
        String::from("sqlite::memory:")
    }

    pub async fn build(&self) -> Result<sqlx::sqlite::SqlitePool, sqlx::Error> {
        let opts = SqliteConnectOptions::from_str(&self.url)?;
        let opts = opts.create_if_missing(true);
        let opts = opts.disable_statement_logging();
        sqlx::sqlite::SqlitePoolOptions::new()
            .min_connections(1)
            .connect_with(opts)
            .await
    }
}
