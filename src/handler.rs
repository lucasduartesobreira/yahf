use std::{marker::PhantomData, pin::Pin};

use async_trait::async_trait;
use futures::Future;
use serde::{de::DeserializeOwned, Serialize};

type StandardBodyType = String;
type GenericRequest = Request<StandardBodyType>;
type GenericResponse = Response<StandardBodyType>;
type BoxedHandler = Box<dyn Fn(GenericRequest) -> Pin<Box<dyn Future<Output = GenericResponse>>>>;
type RefHandler<'a> = &'a dyn Fn(GenericRequest) -> Pin<Box<dyn Future<Output = GenericResponse>>>;

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

pub struct Json<T>(PhantomData<T>);

impl<T> Json<T> {
    fn new() -> Self {
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

fn encapsulate_runner<FnInput, FnOutput, Deserializer, Serializer, R>(
    runner: R,
    _deserializer: &Deserializer,
    _serializer: &Serializer,
) -> impl Fn(Request<String>) -> Pin<Box<dyn Future<Output = Result<Response<String>>>>>
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
) -> Result<Response<String>>
where
    R: Runner<(FnInput, Deserializer), (FnOutput, Serializer)>,
{
    runner.call_runner(req).await
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use async_std_test::async_test;
    use serde::{Deserialize, Serialize};

    use crate::handler::Json;

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
        let a = encapsulate_runner(simple_handler, &Json(PhantomData), &Json(PhantomData));
        let b = a(Request {
            body: serde_json::json!({ "field": "South of the border" }).to_string(),
        })
        .await;

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": "South of the border - Ed Sheeran"  }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_unit_handler_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(unit_handler, &(), &Json(PhantomData));
        let b = a(Request {
            body: serde_json::json!({ "field": "South of the border" }).to_string(),
        })
        .await;

        let expected_field_result = "HOPE - NF";

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_simple_handler_with_body_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(
            simple_handler_with_body,
            &Json(PhantomData),
            &Json(PhantomData),
        );
        let b = a(Request {
            body: serde_json::json!({ "field": "So Good" }).to_string(),
        })
        .await;

        let expected_field_result = "So Good - Halsey";

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_handler_with_simple_body_on_input_and_output_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(
            handler_with_simple_body_on_input_and_output,
            &Json(PhantomData),
            &Json(PhantomData),
        );
        let b = a(Request {
            body: serde_json::json!({ "field": "Sharks" }).to_string(),
        })
        .await;

        let expected_field_result = "Sharks - Imagine Dragons";

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }
}
