use std::net::SocketAddr;

use axum::Extension;
use service::{cache::CachePool, client::ClientManager};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

mod handler;
mod service;
pub mod settings;

pub struct Server {
    address: SocketAddr,
    cache_pool: CachePool,
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
            .route("/api/redirect/:kind", get(handler::api::redirect::handler))
            .route("/api/status", get(handler::api::status::handler))
            .route("/api/user", get(handler::api::user::handler))
            .layer(TraceLayer::new_for_http())
            .layer(Extension(self.cache_pool))
            .layer(Extension(self.client_manager))
    }

    pub async fn listen(self) {
        let listener = TcpListener::bind(self.address).await.unwrap();
        axum::serve(listener, self.router()).await.unwrap()
    }
}
