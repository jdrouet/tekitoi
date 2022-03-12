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
    #[serde(default = "Settings::default_static_path")]
    static_path: PathBuf,
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

    fn default_static_path() -> PathBuf {
        PathBuf::from("./static")
    }
}

impl Settings {
    #[cfg(test)]
    pub fn from_path(path: &str) -> Self {
        let path = std::path::PathBuf::from(path);
        config::Config::builder()
            .add_source(config::File::from(path))
            .add_source(config::Environment::default().separator("__"))
            .build()
            .expect("couldn't build settings")
            .try_deserialize()
            .expect("couldn't deserialize settings")
    }

    pub fn build(config_path: &Option<PathBuf>) -> Self {
        let cfg = config::Config::builder();
        let cfg = match config_path {
            Some(path) => cfg.add_source(config::File::from(path.clone())),
            None => cfg,
        };
        cfg.add_source(config::Environment::default().separator("__"))
            .build()
            .expect("couldn't build settings")
            .try_deserialize()
            .expect("couldn't deserialize settings")
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

    pub fn static_path(&self) -> &PathBuf {
        &self.static_path
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
