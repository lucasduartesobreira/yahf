extern crate yahf;

use yahf::server::Server;

fn main() {
    let a = Server::new().get(
        "/",
        || async { "Hello world".to_string() },
        &(),
        &String::with_capacity(0),
    );

    a.listen("localhost:8000").unwrap();
}
