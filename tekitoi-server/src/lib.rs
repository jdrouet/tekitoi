use std::net::SocketAddr;

use axum::Extension;
use service::client::ClientManager;
use tokio::net::TcpListener;

mod handler;
mod service;
pub mod settings;

pub struct Server {
    address: SocketAddr,
    cache_pool: deadpool_redis::Pool,
    client_manager: ClientManager,
}

impl Server {
    pub fn new(value: settings::Settings) -> Self {
        Self {
            address: value.address(),
            cache_pool: value.build_cache_pool(),
            client_manager: value.build_client_manager(),
        }
    }
}

impl Server {
    fn router(self) -> axum::Router {
        use axum::routing::{get, post};

        axum::Router::new()
            .route("/authorize", get(handler::view::authorize::handler))
            .route("/api/access-token", post(handler::api::token::handler))
            .route(
                "/api/authorize/:kind/:state",
                get(handler::api::authorize::handler),
            )
            .route("/api/redirect/{kind}", get(handler::api::redirect::handler))
            .route("/api/status", get(handler::api::status::handler))
            .route("/api/user", get(handler::api::user::handler))
            .layer(Extension(self.cache_pool))
            .layer(Extension(self.client_manager))
    }

    pub async fn listen(self) {
        let listener = TcpListener::bind(self.address).await.unwrap();
        axum::serve(listener, self.router()).await.unwrap()
    }
}
