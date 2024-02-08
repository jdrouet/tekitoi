use std::net::SocketAddr;

use axum::Extension;
use tokio::net::TcpListener;
use tower_http::{services::ServeDir, trace::TraceLayer};

mod entity;
mod handler;
mod model;
mod service;
pub mod settings;

#[cfg(test)]
fn init_logger() {
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("error: {err:?}");
    }
}

pub struct Server {
    address: SocketAddr,
    static_dir: ServeDir,
    base_url: service::BaseUrl,
    database: service::database::DatabasePool,
}

impl Server {
    pub async fn new(value: settings::Settings) -> Self {
        let address = value.address();
        let base_url = value.base_url();
        let database = value.build_database_pool().await;

        database.migrate().await.expect("couldn't migrate database");

        value
            .applications
            .synchronize(&database)
            .await
            .expect("couldn't synchronize applications");

        Self {
            address,
            base_url,
            database,
            static_dir: ServeDir::new(value.static_path()),
        }
    }
}

impl Server {
    pub fn router(self) -> axum::Router {
        use axum::routing::{get, post};

        axum::Router::new()
            .route("/authorize", get(handler::view::authorize::handler))
            .route("/api/access-token", post(handler::api::token::handler))
            .route(
                "/api/authorize/:request_id/:provider_id",
                get(handler::api::authorize::handler),
            )
            .route("/api/redirect", get(handler::api::redirect::handler))
            .route("/api/status", get(handler::api::status::handler))
            .route("/api/user", get(handler::api::user::handler))
            .layer(Extension(self.database))
            .layer(Extension(self.base_url))
            .nest_service("/static", self.static_dir)
            .layer(TraceLayer::new_for_http())
    }

    pub async fn listen(self) {
        tracing::debug!("starting server on {}", self.address);
        let listener = TcpListener::bind(self.address).await.unwrap();
        axum::serve(listener, self.router()).await.unwrap()
    }
}
