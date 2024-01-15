mod arguments;

use tekitoi_server::Server;

#[tokio::main]
async fn main() {
    let args = arguments::Arguments::build();
    let cfg = args.settings();
    cfg.set_logger();

    let server = Server::new(cfg).await;
    tracing::debug!("starting server");
    server.listen().await;
}
