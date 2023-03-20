use serde::{de::DeserializeOwned, Serialize};

use crate::io::{Error, HttpError, HttpRequest, HttpResponse};

type HandlerResult<T> = Result<T, HttpError>;
pub type HttpResult<T> = HandlerResult<HttpResponse<T>>;
type InternalResult<T> = Result<T, Error>;
type GenericHttpRequest = HttpRequest<String>;
type GenericHttpResponse = InternalResult<HttpResponse<String>>;

pub trait GenericHandlerClosure: Fn(GenericHttpRequest) -> GenericHttpResponse {}
pub type BoxedHandler = Box<dyn GenericHandlerClosure>;

pub trait Runner<Req, Res, Input, Output> {
    fn create_run(self) -> BoxedHandler;
}

impl<F> GenericHandlerClosure for F where F: Fn(GenericHttpRequest) -> GenericHttpResponse {}

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
