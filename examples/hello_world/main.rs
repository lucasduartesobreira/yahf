extern crate yahf;

use std::net::SocketAddr;

use yahf::server_hyper::HyperServer;

#[tokio::main]
async fn main() {
    let a = HyperServer::new().get(
        "/",
        || async { "Hello world".to_string() },
        &(),
        &String::with_capacity(0),
    );

    a.listen(SocketAddr::from(([127, 0, 0, 1], 8000)))
        .await
        .unwrap();
}
