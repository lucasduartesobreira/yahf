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
pub type BoxedAsyncHandler<'a, 'b> =
    Box<dyn 'a + Fn(LocalGenericHttpRequest) -> BoxFuture<'b, GenericHttpResponse>>;

pub trait Runner<Req, Res, Input, Output> {
    fn create_run(self) -> BoxedHandler;
}

type LocalGenericHttpRequest = RequestWrapper<String>;

#[async_trait]
pub trait RealRunner<Input, Output, Extractor, BodyType>: Clone + Sync + Send {
    async fn run(&self, req: LocalGenericHttpRequest) -> GenericHttpResponse;
}

pub trait CRunner<Input, Output, Extractor, BodyType> {
    fn create_runner<'a, 'b>(&'a self, _extractor: Extractor) -> BoxedAsyncHandler<'a, 'b>
    where
        'a: 'b,
        'a: 'static;
}

#[async_trait]
impl<BodyType, Extractor, Req, Res, FFut, F> RealRunner<Req, Res, Extractor, BodyType> for F
where
    F: Fn(Req) -> FFut + Clone + Sync + Send,
    FFut: Future<Output = Res> + Send,
    Req: TryFromWithExtractor<Extractor, BodyType, Req> + Send,
    Res: Into<GenericHttpResponse>,
{
    async fn run(&self, req: LocalGenericHttpRequest) -> GenericHttpResponse {
        self(Req::try_from(req)?).await.into()
    }
}

impl<Req, Res, BodyType, Extractor, F> CRunner<Req, Res, Extractor, BodyType> for F
where
    F: RealRunner<Req, Res, Extractor, BodyType> + 'static,
{
    #[allow(unused_variables)]
    fn create_runner<'a, 'b>(&'a self, extractor: Extractor) -> BoxedAsyncHandler<'a, 'b>
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
    fn extract(content: String) -> Result<Self::Item, String>;
}

#[derive(Clone)]
pub struct Json<T>(PhantomData<T>);

impl<T> BodyExtractors for Json<T>
where
    T: DeserializeOwned,
{
    type Item = T;

    fn extract(content: String) -> Result<Self::Item, String> {
        let deserialized = serde_json::from_str(content.as_str());
        deserialized.map_err(|err| err.to_string())
    }
}

impl<T> Json<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

pub trait TryFromWithExtractor<WithExtractor, BodyType, OutputType> {
    fn try_from(value: LocalGenericHttpRequest) -> Result<OutputType, error::Error>;
}

impl<Req, Extractor> TryFromWithExtractor<Extractor, Req, Req> for Req
where
    Extractor: BodyExtractors<Item = Req>,
    Req: DeserializeOwned,
{
    fn try_from(value: LocalGenericHttpRequest) -> Result<Req, error::Error> {
        let body = value.request.into_body();
        Extractor::extract(body).map_err(error::Error::ParseBody)
    }
}

impl<Req, Extractor> TryFromWithExtractor<Extractor, Req, Request<Req>> for Request<Req>
where
    Extractor: BodyExtractors<Item = Req>,
    Req: DeserializeOwned,
{
    fn try_from(value: LocalGenericHttpRequest) -> Result<Request<Req>, error::Error> {
        let (parts, body) = value.request.into_parts();
        Extractor::extract(body)
            .map(|result| Request::from_parts(parts, result))
            .map_err(error::Error::ParseBody)
    }
}

#[cfg(test)]
mod async_runner {

    use async_std_test::async_test;

    use http::{Request, Response};
    use serde::Deserialize;

    use super::{CRunner, GenericHttpResponse, Json, RequestWrapper};

    #[derive(Deserialize)]
    struct SomeBodyType {
        _correct: bool,
    }

    async fn runner_with_request(_req: Request<SomeBodyType>) -> GenericHttpResponse {
        Ok(Response::new(
            serde_json::json!({"other_new_structure": true}).to_string(),
        ))
    }

    async fn runner_with_simple_struct(_req: SomeBodyType) -> GenericHttpResponse {
        Ok(Response::new(
            serde_json::json!({"new_structure": true}).to_string(),
        ))
    }

    #[async_test]
    async fn test_runner_works() -> std::io::Result<()> {
        let body = serde_json::json!({"_correct": false}).to_string();
        let request = RequestWrapper {
            request: Request::new(body.clone()),
        };

        let request2 = RequestWrapper {
            request: Request::new(body),
        };

        let handler1 = runner_with_simple_struct.create_runner(Json::new());
        let handler2 = runner_with_request.create_runner(Json::new());

        assert_eq!(
            handler1(request).await.unwrap().into_body(),
            serde_json::json!({"new_structure": true}).to_string()
        );

        assert_eq!(
            handler2(request2).await.unwrap().into_body(),
            serde_json::json!({"other_new_structure": true}).to_string()
        );

        Ok(())
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
