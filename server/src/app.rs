use axum::Extension;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::helper::parse_env_or;

pub(crate) struct Config {
    host: std::net::IpAddr,
    port: u16,

    cache: crate::service::cache::Config,
    dataset: crate::service::dataset::Config,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            host: parse_env_or("HOST", IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))?,
            port: parse_env_or("PORT", 3010)?,

            cache: crate::service::cache::Config::from_env()?,
            dataset: crate::service::dataset::Config::from_env()?,
        })
    }

    pub fn build(self) -> anyhow::Result<Application> {
        Ok(Application {
            socket_address: SocketAddr::from((self.host, self.port)),
            cache: self.cache.build()?,
            dataset: self.dataset.build()?,
        })
    }
}

pub(crate) struct Application {
    socket_address: SocketAddr,
    cache: crate::service::cache::Client,
    dataset: crate::service::dataset::Client,
}

impl Application {
    pub async fn run(self) -> anyhow::Result<()> {
        tracing::debug!("binding socket to {}", self.socket_address);
        let listener = TcpListener::bind(self.socket_address).await?;
        let router = crate::router::create()
            .layer(Extension(self.dataset))
            .layer(Extension(self.cache))
            .layer(TraceLayer::new_for_http());
        tracing::info!("listening on {}", self.socket_address);
        axum::serve(listener, router).await?;
        Ok(())
    }
}
