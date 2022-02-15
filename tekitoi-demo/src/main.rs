mod authorize;
mod home;
mod redirect;
mod settings;
mod status;

use actix_web::{web, App, HttpServer};
use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let log_level = std::env::var("LOG")
        .ok()
        .and_then(|value| Level::from_str(&value).ok())
        .unwrap_or(Level::DEBUG);
    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cfg = settings::Settings::build();
    tracing::trace!("loaded configuration {:?}", cfg);
    let oauth_client = web::Data::new(cfg.oauth_client());
    let redis_client = web::Data::new(cfg.redis_client());

    tracing::debug!("starting server");
    HttpServer::new(move || {
        App::new()
            .app_data(oauth_client.clone())
            .app_data(redis_client.clone())
            .service(authorize::handler)
            .service(status::handler)
            .service(redirect::handler)
            .service(home::handler)
    })
    .bind(cfg.address())?
    .workers(1)
    .run()
    .await
}
