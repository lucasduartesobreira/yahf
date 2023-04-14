use std::{marker::PhantomData, pin::Pin};

use async_trait::async_trait;
use futures::Future;
use serde::{de::DeserializeOwned, Serialize};

type StandardBodyType = String;
pub type GenericRequest = Request<StandardBodyType>;
pub type GenericResponse = Response<StandardBodyType>;
pub type BoxedHandler =
    Box<dyn Fn(GenericRequest) -> Pin<Box<dyn Future<Output = GenericResponse>>>>;
pub type RefHandler<'a> =
    &'a dyn Fn(GenericRequest) -> Pin<Box<dyn Future<Output = GenericResponse>>>;

#[derive(Debug)]
pub struct SerdeError {
    body: String,
}

impl SerdeError {
    pub fn new(body: String) -> Self {
        Self { body }
    }
}

impl From<SerdeError> for GenericResponse {
    fn from(val: SerdeError) -> Self {
        GenericResponse::new(val.body)
    }
}

pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

pub struct Request<T> {
    body: T,
    method: Method,
    uri: Uri,
}

pub struct Uri {
    path: String,
    host: String,
}

impl Uri {
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn path_mut(&mut self) -> &mut String {
        &mut self.path
    }

    pub fn host(&self) -> &str {
        self.host.as_str()
    }
}

impl Default for Uri {
    fn default() -> Self {
        Uri {
            path: String::from("/"),
            host: String::from("http://localhost"),
        }
    }
}

type Result<T> = std::result::Result<T, SerdeError>;

impl<T> Request<T> {
    pub fn new(value: T) -> Self {
        Self {
            body: value,
            method: Method::Get,
            uri: Uri::default(),
        }
    }

    pub fn body(&self) -> &T {
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

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.method
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn uri_mut(&mut self) -> &mut Uri {
        &mut self.uri
    }
}

pub struct Response<T> {
    body: T,
}

impl<T> Response<T> {
    pub fn new(value: T) -> Self {
        Self { body: value }
    }

    pub fn body(&self) -> &T {
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

pub trait BodyDeserializer {
    type Item: DeserializeOwned;

    fn deserialize(content: &StandardBodyType) -> Result<Self::Item>
    where
        Self: std::marker::Sized;
}

pub trait BodySerializer {
    type Item;

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
    async fn call_runner(&self, run: Request<StandardBodyType>) -> Response<StandardBodyType>;
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
    async fn call_runner(&self, inp: Request<String>) -> Response<String> {
        match FnIn::try_into(inp) {
            Ok(req) => match FnOut::try_into(self(req).await) {
                Ok(response) => response,
                Err(serde_error) => serde_error.into(),
            },
            Err(serde_error) => serde_error.into(),
        }
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
    async fn call_runner(&self, _inp: Request<String>) -> Response<String> {
        match FnOut::try_into(self().await) {
            Ok(response) => response,
            Err(serde_error) => serde_error.into(),
        }
    }
}

pub struct Json<T>(PhantomData<T>);

impl<T> Json<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

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

impl BodySerializer for String {
    type Item = String;

    fn serialize(content: &Self::Item) -> Result<StandardBodyType> {
        Ok(content.to_owned())
    }
}

pub fn encapsulate_runner<FnInput, FnOutput, Deserializer, Serializer, R>(
    runner: R,
    _deserializer: &Deserializer,
    _serializer: &Serializer,
) -> impl Fn(Request<String>) -> Pin<Box<dyn Future<Output = Response<String>>>>
where
    R: Runner<(FnInput, Deserializer), (FnOutput, Serializer)> + 'static,
    Deserializer: 'static,
    Serializer: 'static,
    FnInput: 'static,
    FnOutput: 'static,
{
    move |request| Box::pin(call_runner(runner.clone(), request))
}

async fn call_runner<FnInput, FnOutput, Deserializer, Serializer, R>(
    runner: R,
    req: Request<String>,
) -> Response<String>
where
    R: Runner<(FnInput, Deserializer), (FnOutput, Serializer)>,
{
    runner.call_runner(req).await
}

#[cfg(test)]
mod tests {

    use async_std_test::async_test;
    use serde::{Deserialize, Serialize};

    use crate::handler::{Json, Method, Uri};

    use super::{encapsulate_runner, Request, Response};

    #[derive(Deserialize, Serialize)]
    struct SomeBodyType {
        field: String,
    }

    async fn simple_handler(input: Request<SomeBodyType>) -> Response<SomeBodyType> {
        let mut new_field = input.body().field.to_owned();
        new_field.push_str(" - Ed Sheeran");

        Response::new(SomeBodyType { field: new_field })
    }

    async fn unit_handler() -> Response<SomeBodyType> {
        let new_field = String::from("HOPE - NF");

        Response::new(SomeBodyType { field: new_field })
    }

    async fn simple_handler_with_body(input: SomeBodyType) -> Response<SomeBodyType> {
        let mut new_field = input.field;
        new_field.push_str(" - Halsey");

        Response::new(SomeBodyType { field: new_field })
    }

    async fn handler_with_simple_body_on_input_and_output(input: SomeBodyType) -> SomeBodyType {
        let mut new_field = input.field;
        new_field.push_str(" - Imagine Dragons");

        SomeBodyType { field: new_field }
    }

    #[async_test]
    async fn test_simple_handler_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(simple_handler, &Json::new(), &Json::new());
        let b = a(Request {
            body: serde_json::json!({ "field": "South of the border" }).to_string(),
            uri: Uri::default(),
            method: Method::Get,
        })
        .await;

        assert_eq!(
            b.body().as_str(),
            serde_json::json!({ "field": "South of the border - Ed Sheeran"  }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_unit_handler_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(unit_handler, &(), &Json::new());
        let b = a(Request {
            body: serde_json::json!({ "field": "South of the border" }).to_string(),
            uri: Uri::default(),
            method: Method::Get,
        })
        .await;

        let expected_field_result = "HOPE - NF";

        assert_eq!(
            b.body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_simple_handler_with_body_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(simple_handler_with_body, &Json::new(), &Json::new());
        let b = a(Request {
            body: serde_json::json!({ "field": "So Good" }).to_string(),
            uri: Uri::default(),
            method: Method::Get,
        })
        .await;

        let expected_field_result = "So Good - Halsey";

        assert_eq!(
            b.body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_handler_with_simple_body_on_input_and_output_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(
            handler_with_simple_body_on_input_and_output,
            &Json::new(),
            &Json::new(),
        );
        let b = a(Request {
            body: serde_json::json!({ "field": "Sharks" }).to_string(),
            uri: Uri::default(),
            method: Method::Get,
        })
        .await;

        let expected_field_result = "Sharks - Imagine Dragons";

        assert_eq!(
            b.body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }
}
