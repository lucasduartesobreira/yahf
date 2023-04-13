use std::{marker::PhantomData, pin::Pin};

use async_trait::async_trait;
use futures::Future;
use http::{Request, Response};
use serde::de::DeserializeOwned;

use crate::error::{self, Error, HttpError};

#[allow(dead_code)]
type HandlerResult<T> = Result<T, HttpError>;
#[allow(dead_code)]
pub type HttpResult<T> = HandlerResult<Response<T>>;
type InternalResult<T> = Result<T, Error>;
pub type GenericHttpResponse = InternalResult<Response<String>>;

pub type BoxedAsyncHandler = Box<
    dyn 'static + Fn(LocalGenericHttpRequest) -> Pin<Box<dyn Future<Output = GenericHttpResponse>>>,
>;

#[allow(dead_code)]
pub type RefAsyncHandler<'a> =
    &'a dyn Fn(
        RequestWrapper<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Response<String>, Error>>>>;

type LocalGenericHttpRequest = RequestWrapper<String>;

#[async_trait]
pub trait RealRunner<Input, Output, Extractor, BodyType>: Clone + Sync + Send {
    async fn run(&self, req: LocalGenericHttpRequest) -> GenericHttpResponse;
}

pub trait CRunner<Input, Output, Extractor, BodyType> {
    fn create_runner(&'static self, _extractor: &Extractor) -> BoxedAsyncHandler;
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
    fn create_runner(&'static self, extractor: &Extractor) -> BoxedAsyncHandler {
        Box::new(move |req: LocalGenericHttpRequest| Box::pin(async { self.run(req).await }))
    }
}

pub struct RequestWrapper<Body> {
    request: Request<Body>,
}

impl<Body> RequestWrapper<Body> {
    pub fn new(data: Body) -> Self {
        Self {
            request: Request::new(data),
        }
    }
}

pub trait BodyExtractors {
    type Item: DeserializeOwned;
    fn extract(content: String) -> Result<Self::Item, String>;
}

#[derive(Clone, Copy)]
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
    #[allow(dead_code)]
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

mod improv {
    #[derive(Debug)]
    pub struct SerdeError {
        body: String,
    }

    impl SerdeError {
        pub fn new(body: String) -> Self {
            Self { body }
        }
    }

    pub struct Request<T> {
        body: T,
    }

    type Result<T> = std::result::Result<T, SerdeError>;

    impl<T> Request<T> {
        fn new(value: T) -> Self {
            Self { body: value }
        }

        fn body(&self) -> &T {
            &self.body
        }

        // TODO: Valuate if this will keep this fn or move to an from_parts style
        fn and_then<BodyType>(
            self,
            callback: impl FnOnce(T) -> Result<BodyType>,
        ) -> Result<Request<BodyType>> {
            let body = self.body;
            callback(body).map(Request::<BodyType>::new)
        }
    }

    pub struct Response<T> {
        body: T,
    }

    impl<T> Response<T> {
        fn new(value: T) -> Self {
            Self { body: value }
        }

        fn body(&self) -> &T {
            &self.body
        }

        // TODO: Valuate if this will keep this fn or move to an from_parts style
        fn and_then<BodyType>(
            self,
            callback: impl FnOnce(T) -> Result<BodyType>,
        ) -> Result<Response<BodyType>> {
            let body = self.body;
            callback(body).map(Response::<BodyType>::new)
        }
    }

    type StandardBodyType = String;

    trait BodyDeserializer {
        type Item: DeserializeOwned;

        fn deserialize(content: &StandardBodyType) -> Result<Self::Item>
        where
            Self: std::marker::Sized;
    }

    trait BodySerializer {
        type Item: Serialize;

        fn serialize(content: &Self::Item) -> Result<StandardBodyType>;
    }

    /// Describes a type that can be extracted using a BodyExtractors
    pub trait RunnerInput<Extractor> {
        fn try_into(input: Request<StandardBodyType>) -> Result<Self>
        where
            Self: std::marker::Sized;
    }

    impl<BodyType, Extractor> RunnerInput<Extractor> for BodyType
    where
        Extractor: BodyDeserializer<Item = BodyType>,
        BodyType: DeserializeOwned,
    {
        fn try_into(input: Request<String>) -> Result<Self>
        where
            Self: std::marker::Sized,
        {
            Extractor::deserialize(input.body())
        }
    }

    impl<BodyType, Extractor> RunnerInput<Extractor> for Request<BodyType>
    where
        Extractor: BodyDeserializer<Item = BodyType>,
        BodyType: DeserializeOwned,
    {
        fn try_into(input: Request<String>) -> Result<Self>
        where
            Self: std::marker::Sized,
        {
            input.and_then(|body| Extractor::deserialize(&body))
        }
    }

    pub trait RunnerOutput<Serializer> {
        fn try_into(self) -> Result<Response<String>>;
    }

    impl<BodyType, Serializer> RunnerOutput<Serializer> for Response<BodyType>
    where
        Serializer: BodySerializer<Item = BodyType>,
        BodyType: Serialize,
    {
        fn try_into(self) -> Result<Response<String>> {
            self.and_then(|body| Serializer::serialize(&body))
        }
    }

    impl<BodyType, Serializer> RunnerOutput<Serializer> for BodyType
    where
        Serializer: BodySerializer<Item = BodyType>,
        BodyType: Serialize,
    {
        fn try_into(self) -> Result<Response<String>> {
            Serializer::serialize(&self).map(Response::new)
        }
    }

    #[async_trait]
    pub trait Runner<Input, Output>: Clone + Send + Sync {
        async fn call_runner(
            &self,
            run: Request<StandardBodyType>,
        ) -> Result<Response<StandardBodyType>>;
    }

    #[async_trait]
    impl<ReqBody, ResBody, FnIn, FnOut, BodyDes, BodySer, Fut, F>
        Runner<(FnIn, BodyDes), (FnOut, BodySer)> for F
    where
        F: Fn(FnIn) -> Fut + Send + Sync + Clone,
        Fut: Future<Output = FnOut> + Send,
        FnIn: RunnerInput<BodyDes> + Send,
        BodyDes: BodyDeserializer<Item = ReqBody>,
        ReqBody: DeserializeOwned,
        FnOut: RunnerOutput<BodySer>,
        BodySer: BodySerializer<Item = ResBody>,
        ResBody: Serialize,
    {
        async fn call_runner(&self, inp: Request<String>) -> Result<Response<String>> {
            FnOut::try_into(self(FnIn::try_into(inp)?).await)
        }
    }

    #[async_trait]
    impl<ResBody, FnOut, BodySer, Fut, F> Runner<((), ()), (FnOut, BodySer)> for F
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: Future<Output = FnOut> + Send,
        FnOut: RunnerOutput<BodySer>,
        BodySer: BodySerializer<Item = ResBody>,
        ResBody: Serialize,
    {
        async fn call_runner(&self, _inp: Request<String>) -> Result<Response<String>> {
            FnOut::try_into(self().await)
        }
    }

    struct Json<T>(PhantomData<T>);

    impl<T> BodyDeserializer for Json<T>
    where
        T: DeserializeOwned,
    {
        type Item = T;

        fn deserialize(content: &StandardBodyType) -> Result<Self::Item>
        where
            Self: std::marker::Sized,
        {
            serde_json::from_str(content).map_err(|err| SerdeError {
                body: err.to_string(),
            })
        }
    }

    impl<T> BodySerializer for Json<T>
    where
        T: Serialize,
    {
        type Item = T;

        fn serialize(content: &Self::Item) -> Result<String> {
            serde_json::to_string(content).map_err(|err| SerdeError {
                body: err.to_string(),
            })
        }
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

        let handler1 = runner_with_simple_struct.create_runner(&Json::new());
        let handler2 = runner_with_request.create_runner(&Json::new());

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
