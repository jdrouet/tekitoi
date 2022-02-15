use crate::service::client::{ClientManager, ClientManagerSettings};
use std::path::PathBuf;
use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, serde::Deserialize)]
pub struct Settings {
    #[serde(default = "Settings::default_host")]
    host: String,
    #[serde(default = "Settings::default_port")]
    port: u16,
    base_url: Option<String>,
    cache: deadpool_redis::Config,
    log_level: Option<String>,
    #[serde(default)]
    pub clients: ClientManagerSettings,
}

impl Settings {
    fn default_host() -> String {
        "localhost".into()
    }

    fn default_port() -> u16 {
        3000
    }
}

impl Settings {
    pub fn build(config_path: &Option<PathBuf>) -> Self {
        let mut cfg = config::Config::new();
        if let Some(path) = config_path {
            cfg.merge(config::File::<config::FileSourceFile>::from(path.as_path()))
                .expect("couldn't merge with configuration file");
        }
        cfg.merge(config::Environment::new().separator("__"))
            .expect("couldn't merge with environment");
        cfg.try_into().expect("couldn't build settings")
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn build_cache_pool(&self) -> deadpool_redis::Pool {
        tracing::trace!("creating cache pool with config {:?}", self.cache);
        self.cache
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .expect("couldn't build cache pool")
    }

    fn base_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", self.host, self.port))
    }

    pub fn build_client_manager(&self) -> ClientManager {
        self.clients
            .build(self.base_url().as_str())
            .expect("couldn't build client manager")
    }

    pub fn set_logger(&self) {
        let level = self
            .log_level
            .as_ref()
            .and_then(|value| match Level::from_str(value.as_str()) {
                Ok(value) => Some(value),
                Err(error) => {
                    eprintln!("unable to parse log level {:?} ({:?})", value, error);
                    None
                }
            })
            .unwrap_or(Level::DEBUG);
        let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    }
}
