extern crate yahf;

use std::time;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde::Serialize;
use yahf::handler::Json;
use yahf::handler::Result;
use yahf::request::Request;

use yahf::response::Response;
use yahf::router::Router;
use yahf::server::Server;

#[derive(Debug, Deserialize, Serialize)]
struct ComputationBody {
    value: u32,
}

async fn log_middleware(req: Result<Request<String>>) -> Result<Request<String>> {
    match req.into_inner() {
        Ok(req) => {
            println!(
                "{} - {} - {}",
                time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Negative time")
                    .as_millis(),
                req.method().as_str(),
                req.uri().path()
            );

            Ok(req).into()
        }
        Err(err) => Err(err).into(),
    }
}

async fn log_error(res: Result<Response<String>>) -> Result<Response<String>> {
    match res.into_inner() {
        Err(err) => {
            println!(
                "{} - {}",
                time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Negative time")
                    .as_millis(),
                err.code(),
            );
            Err(err).into()
        }
        ok => ok.into(),
    }
}

async fn first_computation(req: ComputationBody) -> ComputationBody {
    ComputationBody {
        value: req.value + 1,
    }
}

async fn second_computation(req: ComputationBody) -> ComputationBody {
    ComputationBody {
        value: req.value * 2,
    }
}

fn main() {
    let mut router = Router::new().pre(log_middleware);
    router.get("/first", first_computation, &Json::new(), &Json::new());

    let mut server = Server::new().after(log_error);
    server.get("/second", second_computation, &Json::new(), &Json::new());

    let server = server.router(router);

    server.listen("localhost:3000").unwrap();
}
