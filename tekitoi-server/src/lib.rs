use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use service::base_url::BaseUrl;

mod handler;
mod service;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_port")]
    port: u16,
    #[serde(default = "Config::default_host")]
    host: IpAddr,

    base_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            host: Self::default_host(),
            base_url: None,
        }
    }
}

impl Config {
    fn default_host() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
    }

    fn default_port() -> u16 {
        3000
    }

    pub fn new(path: Option<PathBuf>) -> Self {
        let cfg = config::Config::builder();
        let cfg = match path {
            Some(path) => cfg.add_source(config::File::from(path.clone())),
            None => cfg,
        };
        cfg.add_source(config::Environment::default().separator("__"))
            .build()
            .expect("couldn't build settings")
            .try_deserialize()
            .expect("couldn't deserialize settings")
    }

    pub fn build_base_url(&self) -> BaseUrl {
        match self.base_url {
            Some(ref inner) => BaseUrl::from(inner.clone()),
            None => BaseUrl::from(format!("http://{}:{}", self.host, self.port)),
        }
    }

    pub fn build_socket_addr(&self) -> SocketAddr {
        SocketAddr::from((self.host, self.port))
    }

    pub fn build(self) -> Server {
        Server {
            base_url: self.build_base_url(),
            socket_address: self.build_socket_addr(),
        }
    }
}

pub struct Server {
    base_url: BaseUrl,
    pub socket_address: SocketAddr,
}

impl Server {
    pub fn into_router(self) -> axum::Router {
        use axum::extract::State;
        use axum::routing::get;

        axum::Router::new()
            .route("/api/status", get(handler::api::status::handle))
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .with_state(State(self.base_url))
    }

    pub async fn listen(self) {
        tracing::info!("listening on {:?}", self.socket_address);
        let listener = tokio::net::TcpListener::bind(self.socket_address)
            .await
            .unwrap();
        let app = self.into_router();
        axum::serve(listener, app).await.unwrap();
    }
}
