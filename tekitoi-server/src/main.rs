mod arguments;
mod handler;
mod service;
mod settings;

use actix_web::{web::Data, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = arguments::Arguments::build();
    let cfg = args.settings();
    cfg.set_logger();

    let address = cfg.address();
    let cache_pool = Data::new(cfg.build_cache_pool());
    let client_manager = Data::new(cfg.build_client_manager());

    tracing::debug!("starting server on address {}", address);
    HttpServer::new(move || {
        App::new()
            .app_data(cache_pool.clone())
            .app_data(client_manager.clone())
            .service(handler::api::status::handle)
            .service(handler::api::authorize::handle)
            .service(handler::api::redirect::handle)
            .service(handler::api::token::handle)
            .service(handler::view::authorize::handle)
    })
    .bind(address)?
    .run()
    .await
}
