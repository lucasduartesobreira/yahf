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

#[derive(Debug)]
pub struct HttpError {
    _code: u32,
    _body: String,
}

#[derive(Debug)]
pub enum Error {
    ParseBody(String),
    RequestError(HttpError),
}

type HandlerResult<T> = Result<T, HttpError>;
type HttpResult<T> = HandlerResult<HttpResponse<T>>;
type InternalResult<T> = Result<T, Error>;
type GenericHttpRequest = HttpRequest<String>;
type GenericHttpResponse = InternalResult<HttpResponse<String>>;

type BoxedHandler = Box<dyn Fn(GenericHttpRequest) -> GenericHttpResponse>;

pub trait Runner<Req, Res, Input, Output> {
    fn create_run(self) -> BoxedHandler;
}

impl<Req, Res, T> Runner<Req, Res, (HttpRequest<Req>, HttpResponse<Res>), HttpResult<Res>> for T
where
    Req: DeserializeOwned,
    Res: Serialize,
    T: Fn(HttpRequest<Req>, HttpResponse<Res>) -> HandlerResult<HttpResponse<Res>> + 'static,
{
    fn create_run<'a>(self) -> BoxedHandler {
        let handler = move |request: GenericHttpRequest| -> GenericHttpResponse {
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
                                Err(err) => Err(Error::ParseBody(err.to_string())),
                            }
                        }
                        Err(err) => Err(Error::RequestError(err)),
                    }
                }
                Err(err) => Err(Error::ParseBody(err.to_string())),
            }
        };

        Box::new(handler)
    }
}

impl<Res, T> Runner<(), Res, (), HandlerResult<Res>> for T
where
    Res: Serialize,
    T: Fn() -> HandlerResult<Res> + 'static,
{
    fn create_run<'a>(self) -> BoxedHandler {
        let handler = move |_request: GenericHttpRequest| -> GenericHttpResponse {
            let response = self();
            match response {
                Ok(http_response) => {
                    let serialized_resp_body = serde_json::to_string(&http_response);

                    match serialized_resp_body {
                        Ok(response_bd) => Ok(HttpResponse {
                            body: Some(response_bd),
                        }),
                        Err(err) => Err(Error::ParseBody(err.to_string())),
                    }
                }
                Err(err) => Err(Error::RequestError(err)),
            }
        };

        Box::new(handler)
    }
}

impl<Req, Res, T> Runner<Req, Res, Req, Res> for T
where
    Req: DeserializeOwned,
    Res: Serialize,
    T: Fn(Req) -> HandlerResult<Res> + 'static,
{
    fn create_run<'a>(self) -> BoxedHandler {
        let handler = move |request: GenericHttpRequest| -> GenericHttpResponse {
            let req_deserialized = serde_json::from_str(&request.body);

            match req_deserialized {
                Ok(body) => {
                    let response = self(body);
                    match response {
                        Ok(http_response) => {
                            let serialized_resp_body = serde_json::to_string(&http_response);

                            match serialized_resp_body {
                                Ok(response_bd) => Ok(HttpResponse {
                                    body: Some(response_bd),
                                }),
                                Err(err) => Err(Error::ParseBody(err.to_string())),
                            }
                        }
                        Err(err) => Err(Error::RequestError(err)),
                    }
                }
                Err(err) => Err(Error::ParseBody(err.to_string())),
            }
        };

        Box::new(handler)
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

    pub fn add_handler<Req, Res, Input, Output, R>(&mut self, path: &'static str, handler: R)
    where
        R: 'static + Runner<Req, Res, Input, Output>,
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

        let handler = server.routes.get("/").unwrap();

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
            match response.err().unwrap() {
                crate::Error::ParseBody(message) => message,
                crate::Error::RequestError(error) => error._body,
            }
        );

        assert_eq!(
            response.unwrap().body.unwrap(),
            serde_json::json!({ "correct": true }).to_string()
        );
    }
}
