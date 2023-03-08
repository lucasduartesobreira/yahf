use std::collections::HashMap;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct HttpRequest<ReqBody> {
    body: ReqBody,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HttpResponse<ResBody> {
    body: Option<ResBody>,
}

type HttpResult<T> = Result<HttpResponse<T>, String>;
type GenericHttpRequest = HttpRequest<String>;

type BoxedHandler = Box<dyn FnMut(GenericHttpRequest) -> HttpResult<String>>;

pub trait Runner<Req, Res> {
    fn create_run(self) -> BoxedHandler;
}

impl<Req, Res, T> Runner<Req, Res> for T
where
    Req: DeserializeOwned,
    Res: Serialize,
    T: FnMut(HttpRequest<Req>, HttpResponse<Res>) -> Result<HttpResponse<Res>, String> + 'static,
{
    fn create_run<'a>(mut self) -> BoxedHandler {
        Box::new(move |request| -> Result<HttpResponse<String>, String> {
            let req_deserialized = serde_json::from_str(&request.body);
            let http_resp_a = HttpResponse { body: None };

            match req_deserialized {
                Ok(body) => {
                    let response = self(HttpRequest { body }, http_resp_a);
                    match response {
                        Ok(http_response) => {
                            let serialized_resp_body = serde_json::to_string(&http_response.body);

                            match serialized_resp_body {
                                Ok(response_bd) => Ok(HttpResponse {
                                    body: Some(response_bd),
                                }),
                                Err(err) => Err(err.to_string()),
                            }
                        }
                        Err(err) => Err(err),
                    }
                }
                Err(err) => Err(err.to_string()),
            }
        })
    }
}

pub struct Server {
    routes: HashMap<&'static str, BoxedHandler>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn add_handler<Req, Res, R>(&mut self, path: &'static str, handler: R)
    where
        R: 'static + Runner<Req, Res>,
    {
        self.routes.insert(path, handler.create_run());
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use serde::{Deserialize, Serialize};

    use crate::{HttpRequest, HttpResponse, HttpResult, Server};

    #[derive(Debug, Deserialize, Serialize)]
    struct TestStruct {
        correct: bool,
    }

    fn test_handler_with_req_and_res(
        _req: HttpRequest<TestStruct>,
        _res: HttpResponse<TestStruct>,
    ) -> HttpResult<TestStruct> {
        Ok(HttpResponse {
            body: Some(TestStruct { correct: true }),
        })
    }

    #[test]
    fn test_handler_receiving_req_and_res() {
        let mut server = Server::new();

        server.add_handler("/", test_handler_with_req_and_res);

        assert_eq!(server.routes.len(), 1);

        let handler = server.routes.get_mut("/").unwrap();

        let request = HttpRequest {
            body: serde_json::json!({
                "correct": false
            })
            .to_string(),
        };

        let response = handler(request);

        assert!(
            response.is_ok(),
            "Mensagem de erro: {}",
            response.err().unwrap()
        );

        assert_eq!(
            response.unwrap().body.unwrap(),
            serde_json::json!({ "correct": true }).to_string()
        );
    }
}
