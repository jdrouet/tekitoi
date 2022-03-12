mod arguments;
mod handler;
mod service;
mod settings;

use actix_web::{web::Data, App, HttpServer};

macro_rules! bind_services {
    ($app: expr, $static: expr) => {
        $app.service(crate::handler::api::status::handle)
            .service(crate::handler::api::authorize::handle)
            .service(crate::handler::api::redirect::handle)
            .service(crate::handler::api::token::handle)
            .service(crate::handler::api::user::handle)
            .service(crate::handler::view::authorize::handle)
            .service(actix_files::Files::new("/", $static))
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = arguments::Arguments::build();
    let cfg = args.settings();
    cfg.set_logger();

    let address = cfg.address();
    let cache_pool = Data::new(cfg.build_cache_pool());
    let client_manager = Data::new(cfg.build_client_manager());
    let static_path = cfg.static_path().clone();

    tracing::debug!("starting server on address {}", address);
    HttpServer::new(move || {
        bind_services!(
            App::new()
                .app_data(cache_pool.clone())
                .app_data(client_manager.clone()),
            static_path.clone()
        )
    })
    .bind(address)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use crate::service::client::ClientManager;
    use crate::settings::Settings;
    use actix_http::Request;
    use actix_web::dev::ServiceResponse;
    use actix_web::web::Data;
    use actix_web::App;
    use std::str::FromStr;
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    impl From<Settings> for TestServer {
        fn from(value: Settings) -> Self {
            Self {
                cache_pool: Data::new(value.build_cache_pool()),
                client_manager: Data::new(value.build_client_manager()),
            }
        }
    }

    pub struct TestServer {
        pub cache_pool: Data<deadpool_redis::Pool>,
        pub client_manager: Data<ClientManager>,
    }

    impl TestServer {
        fn init_logger() {
            let level = std::env::var("RUST_LOG")
                .ok()
                .and_then(|value| Level::from_str(value.as_str()).ok())
                .unwrap_or(Level::DEBUG);
            let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
            let _ = tracing::subscriber::set_global_default(subscriber);
        }

        pub fn from_simple() -> Self {
            Self::init_logger();
            Settings::from_path("./tests/simple.toml").into()
        }

        pub async fn execute(&self, req: Request) -> ServiceResponse {
            let app = actix_web::test::init_service(bind_services!(
                App::new()
                    .app_data(self.client_manager.clone())
                    .app_data(self.cache_pool.clone()),
                "./static"
            ))
            .await;
            actix_web::test::call_service(&app, req).await
        }
    }
}
