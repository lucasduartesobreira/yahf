extern crate yahf;

use yahf::server::Server;

#[tokio::main]
async fn main() {
    let server = Server::new().get(
        "/",
        || async { "Hello world".to_string() },
        &(),
        &String::with_capacity(0),
    );

    server
        .listen(([127, 0, 0, 1], 8000).into())
        .await
        .unwrap();
}
