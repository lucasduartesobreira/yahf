mod tls_config;
use tls_config::rustls_config;
use yahf::server::Server;

#[tokio::main]
async fn main() {
    let server = Server::new().get(
        "/",
        || async { "Hello world".to_string() },
        &(),
        &String::with_capacity(0),
    );

    let addr = ([127, 0, 0, 1], 8000).into();

    let listener = rustls_config();
    server
        .listen_rustls(listener, addr)
        .await
        .unwrap();
}
