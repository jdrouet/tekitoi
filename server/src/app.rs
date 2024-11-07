use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::helper::parse_env_or;

pub(crate) struct Config {
    host: std::net::IpAddr,
    port: u16,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            host: parse_env_or("HOST", IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))?,
            port: parse_env_or("PORT", 3010)?,
        })
    }

    pub fn build(self) -> anyhow::Result<Application> {
        Ok(Application {
            socket_address: SocketAddr::from((self.host, self.port)),
        })
    }
}

pub(crate) struct Application {
    socket_address: SocketAddr,
}

impl Application {
    pub async fn run(self) -> anyhow::Result<()> {
        tracing::debug!("binding socket to {}", self.socket_address);
        let listener = TcpListener::bind(self.socket_address).await?;
        let router = crate::router::create().layer(TraceLayer::new_for_http());
        tracing::info!("listening on {}", self.socket_address);
        axum::serve(listener, router).await?;
        Ok(())
    }
}
