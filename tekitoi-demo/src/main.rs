mod authorize;
mod home;
mod redirect;
mod settings;
mod status;

use axum::extract::Extension;
use axum::routing::get;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let log_level = std::env::var("LOG")
        .ok()
        .and_then(|value| Level::from_str(&value).ok())
        .unwrap_or(Level::DEBUG);
    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cfg = settings::Settings::build();
    let address = cfg.address();

    let tcp_listener = TcpListener::bind(address).await.unwrap();

    tracing::debug!("starting server");
    let cache = cfg.cache();

    let app = axum::Router::new()
        .route("/", get(home::handler))
        .route("/api/redirect", get(redirect::handler))
        .route("/api/status", get(status::handler))
        .route("/api/authorize", get(authorize::handler))
        .layer(Extension(cfg.oauth_client()))
        .layer(Extension(Arc::new(cfg)))
        .with_state(cache);

    axum::serve(tcp_listener, app).await?;
    Ok(())
}
