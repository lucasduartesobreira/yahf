use http::{Request, Response};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::{Error, HttpError};

type HandlerResult<T> = Result<T, HttpError>;
pub type HttpResult<T> = HandlerResult<Response<T>>;
type InternalResult<T> = Result<T, Error>;
type GenericHttpRequest = Request<String>;
type GenericHttpResponse = InternalResult<Response<String>>;

pub trait GenericHandlerClosure: Fn(GenericHttpRequest) -> GenericHttpResponse {}
pub type BoxedHandler = Box<dyn GenericHandlerClosure>;

pub trait Runner<Req, Res, Input, Output> {
    fn create_run(self) -> BoxedHandler;
}

impl<F> GenericHandlerClosure for F where F: Fn(GenericHttpRequest) -> GenericHttpResponse {}

impl<Req, Res, T> Runner<Req, Res, (Request<Req>, Response<Res>), HttpResult<Res>> for T
where
    Req: DeserializeOwned,
    Res: Serialize,
    T: Fn(Request<Req>) -> HandlerResult<Response<Res>> + 'static,
{
    fn create_run<'a>(self) -> BoxedHandler {
        let handler = move |request: GenericHttpRequest| -> GenericHttpResponse {
            let (parts, body) = request.into_parts();
            let req_deserialized = serde_json::from_str(&body);

            match req_deserialized {
                Ok(body) => {
                    let response = self(Request::from_parts(parts, body));
                    match response {
                        Ok(http_response) => {
                            let (parts, body) = http_response.into_parts();
                            let serialized_resp_body = serde_json::to_string(&body);

                            match serialized_resp_body {
                                Ok(response_bd) => Ok(Response::from_parts(parts, response_bd)),
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
                        Ok(response_bd) => Ok(Response::new(response_bd)),
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
            let (_, body) = request.into_parts();
            let req_deserialized = serde_json::from_str(&body);

            match req_deserialized {
                Ok(body) => {
                    let response = self(body);
                    match response {
                        Ok(http_response) => {
                            let serialized_resp_body = serde_json::to_string(&http_response);

                            match serialized_resp_body {
                                Ok(response_bd) => Ok(Response::new(response_bd)),
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
