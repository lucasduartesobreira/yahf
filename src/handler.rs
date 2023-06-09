use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
};

use async_trait::async_trait;
use futures::Future;
use serde::{de::DeserializeOwned, Serialize};

use crate::{error::Error, request::Request, response::Response};

type StandardBodyType = String;
pub type GenericRequest = Request<StandardBodyType>;
pub type GenericResponse = Response<StandardBodyType>;
pub type BoxedHandler = Box<
    dyn Fn(GenericRequest) -> Pin<Box<dyn Future<Output = GenericResponse> + Send>> + Sync + Send,
>;
pub type RefHandler<'a> = &'a (dyn Fn(GenericRequest) -> Pin<Box<dyn Future<Output = GenericResponse> + Send>>
         + Sync
         + Send);

pub(crate) type InternalResult<T> = std::result::Result<T, Error>;

pub struct Result<T>(InternalResult<T>);

impl<T> Result<T> {
    pub fn into_inner(self) -> InternalResult<T> {
        self.0
    }
}

impl<T> From<InternalResult<T>> for Result<T> {
    fn from(value: InternalResult<T>) -> Self {
        Result(value)
    }
}

impl<T> From<Result<T>> for InternalResult<T> {
    fn from(value: Result<T>) -> Self {
        value.into_inner()
    }
}

impl<T> AsRef<InternalResult<T>> for Result<T> {
    fn as_ref(&self) -> &InternalResult<T> {
        &self.0
    }
}

impl<T> AsMut<InternalResult<T>> for Result<T> {
    fn as_mut(&mut self) -> &mut InternalResult<T> {
        &mut self.0
    }
}

impl<T> Deref for Result<T> {
    type Target = InternalResult<T>;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Result<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

pub trait BodyDeserializer {
    type Item: DeserializeOwned;

    fn deserialize(content: &StandardBodyType) -> InternalResult<Self::Item>
    where
        Self: std::marker::Sized;
}

pub trait BodySerializer {
    type Item;

    fn serialize(content: &Self::Item) -> InternalResult<StandardBodyType>;
}

/// Describes a type that can be extracted using a BodyExtractors
pub trait RunnerInput<Extractor> {
    fn try_into(input: Request<StandardBodyType>) -> InternalResult<Self>
    where
        Self: std::marker::Sized;
}

impl<BodyType, Extractor> RunnerInput<Extractor> for BodyType
where
    Extractor: BodyDeserializer<Item = BodyType>,
    BodyType: DeserializeOwned,
{
    fn try_into(input: Request<String>) -> InternalResult<Self>
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
    fn try_into(input: Request<String>) -> InternalResult<Self>
    where
        Self: std::marker::Sized,
    {
        input.and_then(|body| Extractor::deserialize(&body))
    }
}

pub trait RunnerOutput<Serializer> {
    fn try_into(self) -> InternalResult<Response<String>>;
}

impl<BodyType, Serializer> RunnerOutput<Serializer> for Response<BodyType>
where
    Serializer: BodySerializer<Item = BodyType>,
    BodyType: Serialize,
{
    fn try_into(self) -> InternalResult<Response<String>> {
        self.and_then(|body| Serializer::serialize(&body))
    }
}

impl<BodyType, Serializer> RunnerOutput<Serializer> for BodyType
where
    Serializer: BodySerializer<Item = BodyType>,
    BodyType: Serialize,
{
    fn try_into(self) -> InternalResult<Response<String>> {
        Serializer::serialize(&self).map(Response::new)
    }
}

impl<BodyType, Serializer, BasicRunnerOutput> RunnerOutput<Serializer> for Result<BasicRunnerOutput>
where
    Serializer: BodySerializer<Item = BodyType>,
    BodyType: Serialize,
    BasicRunnerOutput: RunnerOutput<Serializer>,
{
    fn try_into(self) -> InternalResult<Response<String>> {
        self.0.and_then(|resp| BasicRunnerOutput::try_into(resp))
    }
}

#[async_trait]
pub trait Runner<Input, Output>: Clone + Send + Sync {
    async fn call_runner(
        &self,
        run: Request<StandardBodyType>,
    ) -> InternalResult<Response<StandardBodyType>>;
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
    async fn call_runner(&self, inp: Request<String>) -> InternalResult<Response<String>> {
        match FnIn::try_into(inp) {
            Ok(req) => FnOut::try_into(self(req).await),
            Err(serde_error) => Err(serde_error),
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
    async fn call_runner(&self, _inp: Request<String>) -> InternalResult<Response<String>> {
        FnOut::try_into(self().await)
    }
}

pub struct Json<T>(PhantomData<T>);

impl<T> Json<T> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Default for Json<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BodyDeserializer for Json<T>
where
    T: DeserializeOwned,
{
    type Item = T;

    fn deserialize(content: &StandardBodyType) -> InternalResult<Self::Item>
    where
        Self: std::marker::Sized,
    {
        serde_json::from_str(content).map_err(|err| Error::new(err.to_string(), 422))
    }
}

impl<T> BodySerializer for Json<T>
where
    T: Serialize,
{
    type Item = T;

    fn serialize(content: &Self::Item) -> InternalResult<String> {
        serde_json::to_string(content).map_err(|err| Error::new(err.to_string(), 422))
    }
}

impl BodyDeserializer for String {
    type Item = String;

    fn deserialize(_content: &StandardBodyType) -> InternalResult<Self::Item>
    where
        Self: std::marker::Sized,
    {
        Ok(_content.to_owned())
    }
}

impl BodySerializer for String {
    type Item = String;

    fn serialize(content: &Self::Item) -> InternalResult<StandardBodyType> {
        Ok(content.to_owned())
    }
}

pub(crate) fn encapsulate_runner<FnInput, FnOutput, Deserializer, Serializer, R>(
    runner: R,
    _deserializer: &Deserializer,
    _serializer: &Serializer,
) -> impl Fn(Request<String>) -> Pin<Box<dyn Future<Output = Response<String>> + Send>> + Sync
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
    match runner.call_runner(req).await {
        Ok(resp) => resp,
        Err(err) => err.into(),
    }
}

#[cfg(test)]
mod tests {

    use async_std_test::async_test;
    use serde::{Deserialize, Serialize};

    use crate::handler::Json;

    use super::{encapsulate_runner, Request, Response, Result};

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

    async fn unit_handler_with_response_body() -> SomeBodyType {
        let new_field = String::from("HOPE - NF");

        SomeBodyType { field: new_field }
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

    async fn handler_with_simple_body_on_input_and_cf_output(
        input: SomeBodyType,
    ) -> Result<SomeBodyType> {
        let mut new_field = input.field;
        new_field.push_str(" - Eminem");

        Ok(SomeBodyType { field: new_field }).into()
    }

    #[async_test]
    async fn test_simple_handler_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(simple_handler, &Json::new(), &Json::new());
        let c = Request::builder()
            .body(serde_json::json!({ "field": "South of the border" }).to_string());
        let b = a(c).await;

        assert_eq!(
            b.body().as_str(),
            serde_json::json!({ "field": "South of the border - Ed Sheeran"  }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_unit_handler_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(unit_handler, &(), &Json::new());
        let c = Request::builder()
            .body(serde_json::json!({ "field": "South of the border" }).to_string());
        let b = a(c).await;

        let expected_field_result = "HOPE - NF";

        assert_eq!(
            b.body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_unit_handler_with_response_body_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(unit_handler_with_response_body, &(), &Json::new());
        let c = Request::builder()
            .body(serde_json::json!({ "field": "South of the border" }).to_string());
        let b = a(c).await;

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
        let c = Request::builder().body(serde_json::json!({ "field": "So Good" }).to_string());
        let b = a(c).await;

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
        let c = Request::builder().body(serde_json::json!({ "field": "Sharks" }).to_string());
        let b = a(c).await;

        let expected_field_result = "Sharks - Imagine Dragons";

        assert_eq!(
            b.body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_handler_with_simple_body_on_input_and_cf_output_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(
            handler_with_simple_body_on_input_and_cf_output,
            &Json::new(),
            &Json::new(),
        );
        let c = Request::builder().body(serde_json::json!({ "field": "Venom" }).to_string());
        let b = a(c).await;

        let expected_field_result = "Venom - Eminem";

        assert_eq!(
            b.body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }
}
