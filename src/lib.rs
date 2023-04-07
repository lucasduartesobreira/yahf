mod error;
mod handle_selector;
mod handler;

use crate::handler::{BodyExtractors, CRunner, RealRunner};
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

    pub fn add_handler<Req, Res, BodyType, Extractor, R>(
        &mut self,
        path: &'static str,
        handler: &'static R,
        extractor: Extractor,
    ) where
        R: 'static + RealRunner<Req, Res, Extractor, BodyType>,
        Extractor: BodyExtractors<Item = BodyType>,
    {
        self.handler_selector
            .insert(path, handler.create_runner(extractor));
    }
}

#[cfg(test)]
mod tests {

    use async_std_test::async_test;
    use http::{Request, Response};
    use serde::{Deserialize, Serialize};

    use crate::{
        error::Error,
        handler::{GenericHttpResponse, Json, RequestWrapper},
        Server,
    };

    #[derive(Debug, Deserialize, Serialize, Default)]
    struct TestStruct {
        correct: bool,
    }

    async fn test_handler_with_req_and_res(_req: Request<TestStruct>) -> GenericHttpResponse {
        Ok(Response::new(
            serde_json::to_string(&TestStruct { correct: true }).unwrap(),
        ))
    }

    #[async_test]
    async fn test_handler_receiving_req_and_res() -> std::io::Result<()> {
        let mut server = Server::new();

        server.add_handler("/aaaa/bbbb", &test_handler_with_req_and_res, Json::new());

        let handler = server.handler_selector.get("/aaaa/bbbb");

        assert!(handler.is_some());

        let unwraped_handler = handler.unwrap();

        let request = RequestWrapper::new(serde_json::json!({ "correct": false }).to_string());

        let response = unwraped_handler(request).await;

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

        Ok(())
    }
}
