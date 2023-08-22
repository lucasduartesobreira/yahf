use std::{marker::PhantomData, pin::Pin};

use futures::Future;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    deserializer::BodyDeserializer, request::Request, response::Response, result::InternalResult,
    runner_input::RunnerInput, runner_output::RunnerOutput, serializer::BodySerializer,
};

pub(crate) type StandardBodyType = String;
pub type GenericRequest = Request<StandardBodyType>;
pub type GenericResponse = Response<StandardBodyType>;
pub type BoxedHandler = Box<dyn BoxedRunner>;
pub type RefHandler<'a> = &'a (dyn BoxedRunner);

/// An trait to mark functions handler
///
/// To accept new types of handler just impl this trait.
/// All implementations from this crate are using the signature `(Type, BodyDeserializer)` for both generic parameters
pub trait Runner<Input, Output>: Clone + Send + Sync {
    fn call_runner(
        &'_ self,
        run: InternalResult<Request<StandardBodyType>>,
    ) -> impl Future<Output = InternalResult<Response<String>>> + Send + '_;
}

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
    #[allow(clippy::manual_async_fn)]
    fn call_runner(
        &'_ self,
        inp: InternalResult<Request<StandardBodyType>>,
    ) -> impl Future<Output = InternalResult<Response<String>>> + Send + '_ {
        async move {
            let inp = FnIn::try_into(inp);

            match inp {
                Ok(req) => FnOut::try_into(self(req).await),
                Err(err) => Err(err),
            }
        }
    }
}

impl<ResBody, FnOut, BodySer, Fut, F> Runner<((), ()), (FnOut, BodySer)> for F
where
    F: Fn() -> Fut + Send + Sync + Clone,
    Fut: Future<Output = FnOut> + Send,
    FnOut: RunnerOutput<BodySer>,
    BodySer: BodySerializer<Item = ResBody>,
    ResBody: Serialize,
{
    #[allow(clippy::manual_async_fn)]
    fn call_runner(
        &'_ self,
        _run: InternalResult<Request<StandardBodyType>>,
    ) -> impl Future<Output = InternalResult<Response<String>>> + Send + '_ {
        async move {
            _run?;
            FnOut::try_into(self().await)
        }
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

pub trait BoxedRunner: DynClone + Sync + Send {
    fn call(
        &self,
        req: InternalResult<GenericRequest>,
    ) -> Pin<Box<dyn Future<Output = InternalResult<GenericResponse>> + Send>>;
}

impl<F> BoxedRunner for F
where
    F: Fn(
            InternalResult<Request<String>>,
        ) -> Pin<Box<dyn Future<Output = InternalResult<Response<String>>> + Send>>
        + Sync
        + DynClone
        + Send,
{
    fn call(
        &self,
        req: InternalResult<GenericRequest>,
    ) -> Pin<Box<dyn Future<Output = InternalResult<GenericResponse>> + Send>> {
        self(req)
    }
}

pub trait DynClone {
    fn clone_box(&self) -> Box<dyn BoxedRunner>;
}

impl<F> DynClone for F
where
    F: Fn(
            InternalResult<Request<String>>,
        ) -> Pin<Box<dyn Future<Output = InternalResult<Response<String>>> + Send>>
        + Sync
        + Clone
        + Send
        + 'static,
{
    fn clone_box(&self) -> Box<dyn BoxedRunner> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn BoxedRunner> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl Runner<(Request<String>, String), (Response<String>, String)> for Box<dyn BoxedRunner> {
    #[allow(clippy::manual_async_fn)]
    fn call_runner(
        &'_ self,
        run: InternalResult<Request<StandardBodyType>>,
    ) -> impl Future<Output = InternalResult<Response<String>>> + Send + '_ {
        async move { self.call(run).await }
    }
}

pub(crate) fn encapsulate_runner<FnInput, FnOutput, Deserializer, Serializer, R>(
    runner: R,
    _deserializer: &Deserializer,
    _serializer: &Serializer,
) -> impl Fn(
    InternalResult<Request<String>>,
) -> Pin<Box<dyn Future<Output = InternalResult<Response<String>>> + Send>>
       + Sync
       + DynClone
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
    req: InternalResult<Request<String>>,
) -> InternalResult<Response<String>>
where
    R: Runner<(FnInput, Deserializer), (FnOutput, Serializer)>,
{
    runner.call_runner(req).await
}

#[cfg(test)]
mod tests {

    use serde::{Deserialize, Serialize};

    use crate::handler::Json;
    use crate::result::Result;

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

    #[tokio::test]
    async fn test_simple_handler_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(simple_handler, &Json::new(), &Json::new());
        let c = Request::builder()
            .body(serde_json::json!({ "field": "South of the border" }).to_string());
        let b = a(c.into()).await;

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": "South of the border - Ed Sheeran"  }).to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_unit_handler_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(unit_handler, &(), &Json::new());
        let c = Request::builder()
            .body(serde_json::json!({ "field": "South of the border" }).to_string());
        let b = a(c.into()).await;

        let expected_field_result = "HOPE - NF";

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_unit_handler_with_response_body_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(unit_handler_with_response_body, &(), &Json::new());
        let c = Request::builder()
            .body(serde_json::json!({ "field": "South of the border" }).to_string());
        let b = a(c.into()).await;

        let expected_field_result = "HOPE - NF";

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_simple_handler_with_body_implements_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(simple_handler_with_body, &Json::new(), &Json::new());
        let c = Request::builder().body(serde_json::json!({ "field": "So Good" }).to_string());
        let b = a(c.into()).await;

        let expected_field_result = "So Good - Halsey";

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_handler_with_simple_body_on_input_and_output_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(
            handler_with_simple_body_on_input_and_output,
            &Json::new(),
            &Json::new(),
        );
        let c = Request::builder().body(serde_json::json!({ "field": "Sharks" }).to_string());
        let b = a(c.into()).await;

        let expected_field_result = "Sharks - Imagine Dragons";

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_handler_with_simple_body_on_input_and_cf_output_runner() -> std::io::Result<()> {
        let a = encapsulate_runner(
            handler_with_simple_body_on_input_and_cf_output,
            &Json::new(),
            &Json::new(),
        );
        let c = Request::builder().body(serde_json::json!({ "field": "Venom" }).to_string());
        let b = a(c.into()).await;

        let expected_field_result = "Venom - Eminem";

        assert_eq!(
            b.unwrap().body().as_str(),
            serde_json::json!({ "field": expected_field_result }).to_string()
        );

        Ok(())
    }
}
