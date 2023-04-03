use std::marker::PhantomData;

use async_trait::async_trait;
use futures::{future::BoxFuture, Future};
use http::{Request, Response};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::{self, Error, HttpError};

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

type LocalGenericHttpRequest = RequestWrapper<String>;

#[async_trait]
pub trait RealRunner<Input, Output>: Clone + Sync + Send {
    async fn run(&self, req: LocalGenericHttpRequest) -> GenericHttpResponse;
}

pub trait CRunner<Args> {
    fn create_runner<'a, 'b>(
        &'static self,
    ) -> Box<dyn 'a + Fn(LocalGenericHttpRequest) -> BoxFuture<'b, GenericHttpResponse>>
    where
        'a: 'b,
        'a: 'static;
}

#[async_trait]
impl<Req, Res, FFut, F> RealRunner<Req, Res> for F
where
    F: Fn(Req) -> FFut + Clone + Sync + Send,
    FFut: Future<Output = Res> + Send,
    Req: From<LocalGenericHttpRequest> + Send,
    Res: Into<GenericHttpResponse>,
{
    async fn run(&self, req: LocalGenericHttpRequest) -> GenericHttpResponse {
        self(Req::from(req)).await.into()
    }
}

impl<Req, Res, F> CRunner<(Req, Res)> for F
where
    F: RealRunner<Req, Res>,
{
    fn create_runner<'a, 'b>(
        &'static self,
    ) -> Box<dyn 'a + Fn(LocalGenericHttpRequest) -> BoxFuture<'b, GenericHttpResponse>>
    where
        'a: 'b,
        'a: 'static,
    {
        Box::new(move |req: LocalGenericHttpRequest| Box::pin(async { self.run(req).await }))
    }
}

pub struct RequestWrapper<Body> {
    request: Request<Body>,
}

trait BodyExtractors {
    type Item: DeserializeOwned;
    fn extracted(self) -> Self::Item;

    fn extract(content: String) -> Result<Self, String>
    where
        Self: Sized;
}

#[derive(Clone)]
pub struct Json<T>(T);

impl<T> BodyExtractors for Json<T>
where
    T: DeserializeOwned,
{
    type Item = T;
    fn extracted(self) -> T {
        self.0
    }

    fn extract(content: String) -> Result<Self, String> {
        let deserialized = serde_json::from_str(content.as_str());
        deserialized
            .map(|des| Json(des))
            .map_err(|err| err.to_string())
    }
}

impl From<LocalGenericHttpRequest> for Request<String> {
    fn from(value: LocalGenericHttpRequest) -> Self {
        value.request
    }
}

impl<Ext, Body> From<RequestWrapper<String>> for Request<Ext>
where
    Ext: BodyExtractors<Item = Body>,
    Body: DeserializeOwned,
{
    fn from(value: RequestWrapper<String>) -> Self {
        let (parts, body) = value.request.into_parts();
        let des_body = Ext::extract(body);
        Request::from_parts(parts, des_body.unwrap())
    }
}

impl<Body> From<RequestWrapper<String>> for Json<Body>
where
    Body: DeserializeOwned,
{
    fn from(value: RequestWrapper<String>) -> Self {
        let body = value.request.into_body();
        let des_body = Self::extract(body);
        des_body.unwrap()
    }
}

#[cfg(test)]
mod testttt {
    use std::collections::HashMap;

    use http::Request;
    use serde::Deserialize;

    use super::{CRunner, GenericHttpResponse, Json, RequestWrapper};

    #[derive(Deserialize)]
    struct Slaa {
        _correct: bool,
    }

    async fn real_runner(_req: Request<String>) -> GenericHttpResponse {
        todo!()
    }

    async fn rr2(_req: Json<Slaa>) -> GenericHttpResponse {
        todo!()
    }

    #[test]
    fn test_aha() {
        let _a: Request<_> = Request::<Json<Slaa>>::from(RequestWrapper {
            request: Request::new("{\"correct\":true}".to_string()),
        });

        let mut map = HashMap::new();
        map.insert("a", rr2.create_runner());
        map.insert("b", real_runner.create_runner());
    }
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
                        //TODO: Populate headers and other fields with request fields
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
                                //TODO: Populate headers and other fields with request fields
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

#[derive(Clone, Debug, Default)]
pub struct Handler<Req, Res, Signature, F>
where
    F: Sla<Req, Res, Signature>,
{
    runner: F,
    _pdt: PhantomData<(Req, Res, Signature)>,
}

#[allow(dead_code)]
impl<Req, Res, F> Handler<Req, Res, (Req, Res), F>
where
    Req: DeserializeOwned,
    Res: Serialize,
    F: Sla<Req, Res, (Req, Res)>,
{
    pub fn new(runner: F) -> Self {
        Self {
            runner,
            _pdt: PhantomData,
        }
    }

    pub fn pre_hook<AReq, AF>(
        self,
        callback: AF,
    ) -> Handler<AReq, Res, (AReq, Res), impl Sla<AReq, Res, (AReq, Res)>>
    where
        AReq: DeserializeOwned,
        Res: Serialize,
        AF: Fn(AReq) -> Req + Clone,
    {
        let runner = move |request: AReq| {
            let af_resp = callback(request);
            let runner = self.runner.clone();
            runner.run(af_resp)
        };

        Handler::new(runner)
    }

    pub fn sla(self) -> impl Fn(GenericHttpRequest) -> GenericHttpResponse {
        move |request| {
            // TODO: Push parts into Response parts
            let (_parts, body) = request.into_parts();
            let parsed_req_body = serde_json::from_str(&body);

            match parsed_req_body {
                Ok(req_body) => {
                    let handler = self.runner.clone();
                    let response = handler.run(req_body);
                    let serialized_response = serde_json::to_string(&response);
                    match serialized_response {
                        Ok(res) => Ok(Response::new(res)),
                        Err(err) => Err(error::Error::ParseBody(err.to_string())),
                    }
                }
                Err(err) => Err(error::Error::ParseBody(err.to_string())),
            }
        }
    }
}

pub trait Sla<Req, Res, Signature>: Clone {
    fn run(&self, req: Req) -> Res;
}

impl<Req, Res, F> Sla<Req, Res, (Req, Res)> for F
where
    Req: DeserializeOwned,
    Res: Serialize,
    F: Fn(Req) -> Res + Clone,
{
    fn run(&self, req: Req) -> Res {
        self(req)
    }
}

impl<Req, Res, R> Runner<Req, Res, Req, Res> for Handler<Req, Res, (Req, Res), R>
where
    Req: DeserializeOwned,
    Res: Serialize,
    R: Sla<Req, Res, (Req, Res)> + 'static,
{
    fn create_run(self) -> BoxedHandler {
        let handler = move |request: GenericHttpRequest| {
            let (_parts, body) = request.into_parts();
            let parsed_req_body = serde_json::from_str(&body);

            match parsed_req_body {
                Ok(req_body) => {
                    let handler = self.runner.clone();
                    let response = handler.run(req_body);
                    let serialized_response = serde_json::to_string(&response);
                    match serialized_response {
                        Ok(res) => Ok(Response::new(res)),
                        Err(err) => Err(error::Error::ParseBody(err.to_string())),
                    }
                }
                Err(err) => Err(error::Error::ParseBody(err.to_string())),
            }
        };

        Box::new(handler)
    }
}
