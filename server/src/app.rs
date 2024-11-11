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
    fn router(&self) -> axum::Router {
        crate::router::create()
            .layer(Extension(self.dataset.clone()))
            .layer(Extension(self.cache.clone()))
            .layer(TraceLayer::new_for_http())
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tracing::debug!("binding socket to {}", self.socket_address);
        let listener = TcpListener::bind(self.socket_address).await?;
        tracing::info!("listening on {}", self.socket_address);
        axum::serve(listener, self.router()).await?;
        Ok(())
    }
}

#[cfg(test)]
impl Application {
    pub(crate) fn test() -> Self {
        Self {
            socket_address: SocketAddr::from((Ipv4Addr::new(127, 0, 0, 1), 8080)),
            cache: crate::service::cache::Client::test(),
            dataset: crate::service::dataset::Client::test(),
        }
    }

    pub(crate) fn random() -> (u16, Self) {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let port: u16 = rng.gen_range(9000..9100);
        (
            port,
            Self {
                socket_address: SocketAddr::from((Ipv4Addr::new(127, 0, 0, 1), port)),
                cache: crate::service::cache::Client::test(),
                dataset: crate::service::dataset::Client::test(),
            },
        )
    }

    pub(crate) fn cache(&self) -> &crate::service::cache::Client {
        &self.cache
    }

    pub(crate) async fn handle(
        &self,
        req: axum::http::Request<axum::body::Body>,
    ) -> axum::http::Response<axum::body::Body> {
        use tower::ServiceExt;

        self.router().oneshot(req).await.unwrap()
    }
}
