mod arguments;

use tekitoi::Server;

#[tokio::main]
async fn main() {
    let args = arguments::Arguments::build();
    let cfg = args.settings();
    cfg.set_logger();

    let server = Server::new(cfg).await;
    server.listen().await;
}
