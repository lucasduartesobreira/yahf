mod error;
mod handle_selector;
mod handler;

use handle_selector::HandlerSelect;

#[derive(Default)]
pub struct Server<'a> {
    handler_selector: HandlerSelect<'a>,
}

impl<'a> Server<'a> {
    pub fn new() -> Self {
        Self {
            handler_selector: HandlerSelect::new(),
        }
    }

    pub fn add_handler<Req, Res, Input, Output, R>(&mut self, path: &'static str, handler: R)
    where
        R: 'static + handler::Runner<Req, Res, Input, Output>,
    {
        self.handler_selector.insert(path, handler.create_run());
    }
}

#[cfg(test)]
mod tests {

    use http::{Request, Response};
    use serde::{Deserialize, Serialize};

    use crate::{
        error::Error,
        handler::{Handler, HttpResult},
        Server,
    };

    #[derive(Debug, Deserialize, Serialize)]
    struct TestStruct {
        correct: bool,
    }

    fn test_handler_with_req_and_res(_req: Request<TestStruct>) -> HttpResult<TestStruct> {
        Ok(Response::new(TestStruct { correct: true }))
    }

    fn test_handler_with_req_and_res2(_req: TestStruct) -> TestStruct {
        _req
    }

    #[test]
    fn test_handler_receiving_req_and_res() {
        let mut server = Server::new();

        server.add_handler("/aaaa/bbbb", test_handler_with_req_and_res);
        server.add_handler("/bbbb/cccc", {
            Handler::new(test_handler_with_req_and_res2)
                .pre_hook(|_input: TestStruct| TestStruct { correct: true })
        });

        let handler = server.handler_selector.get("/aaaa/bbbb");

        assert!(handler.is_some());

        let unwraped_handler = handler.unwrap();

        let request = Request::builder()
            .body(serde_json::json!({ "correct": false }).to_string())
            .unwrap();

        let response = unwraped_handler(request);

        assert!(
            response.is_ok(),
            "Mensagem de erro: {}",
            match response.err().unwrap() {
                Error::ParseBody(message) => message,
                Error::RequestError(error) => error._body,
            }
        );

        assert_eq!(
            *response.unwrap().body(),
            serde_json::json!({ "correct": true }).to_string()
        );

        let handler = server.handler_selector.get("/bbbb/cccc");

        assert!(handler.is_some());

        let unwraped_handler = handler.unwrap();

        let request = Request::builder()
            .body(serde_json::json!({ "correct": true }).to_string())
            .unwrap();

        let response = unwraped_handler(request);

        assert!(
            response.is_ok(),
            "Mensagem de erro: {}",
            match response.err().unwrap() {
                Error::ParseBody(message) => message,
                Error::RequestError(error) => error._body,
            }
        );

        assert_eq!(
            *response.unwrap().body(),
            serde_json::json!({ "correct": true }).to_string()
        );
    }
}
