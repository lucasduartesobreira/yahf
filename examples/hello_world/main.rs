extern crate yahf;

use yahf::server::Server;

fn main() {
    let mut a = Server::new();

    a.get(
        "/",
        || async { "Hello world".to_string() },
        &(),
        &String::with_capacity(0),
    );

    a.listen("127.0.0.1:8000").unwrap();
}
