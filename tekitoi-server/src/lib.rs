use std::net::SocketAddr;

use axum::Extension;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

mod handler;
pub mod model;
mod service;
pub mod settings;

pub struct Server {
    address: SocketAddr,
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
            .layer(TraceLayer::new_for_http())
            .layer(Extension(self.database))
            .layer(Extension(self.base_url))
    }

    pub async fn listen(self) {
        let listener = TcpListener::bind(self.address).await.unwrap();
        axum::serve(listener, self.router()).await.unwrap()
    }
}
