use axum::Extension;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::helper::parse_env_or;

pub(crate) struct Config {
    host: std::net::IpAddr,
    port: u16,

    database: crate::service::database::Config,
    dataset: crate::service::dataset::Config,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            host: parse_env_or("HOST", IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))?,
            port: parse_env_or("PORT", 3010)?,

            database: crate::service::database::Config::from_env()?,
            dataset: crate::service::dataset::Config::from_env()?,
        })
    }

    pub async fn build(self) -> anyhow::Result<Application> {
        let database = self.database.build().await?;
        database.upgrade().await?;

        self.dataset.synchronize(&database).await?;

        Ok(Application {
            socket_address: SocketAddr::from((self.host, self.port)),
            database,
        })
    }
}

pub(crate) struct Application {
    socket_address: SocketAddr,
    database: crate::service::database::Pool,
}

impl Application {
    fn router(&self) -> axum::Router {
        crate::router::create()
            .layer(Extension(self.database.clone()))
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
    pub(crate) async fn test() -> Self {
        let database = crate::service::database::Config::default()
            .build()
            .await
            .unwrap();
        database.upgrade().await.unwrap();

        crate::service::dataset::RootConfig::test()
            .synchronize(&database)
            .await
            .unwrap();

        Self {
            socket_address: SocketAddr::from((Ipv4Addr::new(127, 0, 0, 1), 8080)),
            database,
        }
    }

    pub(crate) fn database(&self) -> &sqlx::SqlitePool {
        self.database.as_ref()
    }

    pub(crate) async fn handle(
        &self,
        req: axum::http::Request<axum::body::Body>,
    ) -> axum::http::Response<axum::body::Body> {
        use tower::ServiceExt;

        self.router().oneshot(req).await.unwrap()
    }
}
