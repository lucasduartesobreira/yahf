extern crate yahf;

use std::net::SocketAddr;

use yahf::server::Server;

#[tokio::main]
async fn main() {
    let a = Server::new().get(
        "/",
        || async { "Hello world".to_string() },
        &(),
        &String::with_capacity(0),
    );

    a.listen(SocketAddr::from(([127, 0, 0, 1], 8000)))
        .await
        .unwrap();
}
