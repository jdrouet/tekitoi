use crate::service::client::ApplicationCollectionConfig;
use crate::service::database::DatabaseConfig;
use crate::service::BaseUrl;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, serde::Deserialize)]
pub struct Settings {
    #[serde(default = "Settings::default_host")]
    host: IpAddr,
    #[serde(default = "Settings::default_port")]
    port: u16,
    #[serde(default = "Settings::default_static_path")]
    static_path: PathBuf,
    base_url: Option<String>,
    #[serde(default)]
    database: DatabaseConfig,
    #[serde(default)]
    pub applications: ApplicationCollectionConfig,
    log_level: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            host: Self::default_host(),
            port: Self::default_port(),
            static_path: Self::default_static_path(),
            base_url: None,
            database: Default::default(),
            applications: Default::default(),
            log_level: Some("INFO".into()),
        }
    }
}

impl Settings {
    fn default_host() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
    }

    fn default_port() -> u16 {
        3000
    }

    fn default_static_path() -> PathBuf {
        PathBuf::from("./static")
    }
}

impl Settings {
    pub fn build(config_path: Option<PathBuf>) -> Self {
        let cfg = ::config::Config::builder();
        let cfg = match config_path {
            Some(path) => cfg.add_source(config::File::from(path)),
            None => cfg,
        };
        cfg.add_source(::config::Environment::default().separator("__"))
            .build()
            .expect("couldn't build settings")
            .try_deserialize()
            .expect("couldn't deserialize settings")
    }

    pub fn address(&self) -> SocketAddr {
        SocketAddr::from((self.host, self.port))
    }

    pub async fn build_database_pool(&self) -> crate::service::database::DatabasePool {
        self.database
            .build()
            .await
            .expect("couldn't build database pool")
    }

    pub fn base_url(&self) -> BaseUrl {
        BaseUrl::from(
            self.base_url
                .clone()
                .unwrap_or_else(|| format!("http://{}:{}", self.host, self.port)),
        )
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
