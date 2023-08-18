use serde::de::DeserializeOwned;

use crate::{
    handler::{InternalResult, StandardBodyType},
    request::Request,
};

/// Describes a type that can be extracted using a BodyExtractors
pub trait RunnerInput<Extractor> {
    fn try_into(input: InternalResult<Request<StandardBodyType>>) -> InternalResult<Self>
    where
        Self: std::marker::Sized;
}

impl<BodyType, Extractor> RunnerInput<Extractor> for BodyType
where
    Extractor: BodyDeserializer<Item = BodyType>,
    BodyType: DeserializeOwned,
{
    fn try_into(input: InternalResult<Request<String>>) -> InternalResult<Self>
    where
        Self: std::marker::Sized,
    {
        input.and_then(|input| Extractor::deserialize(input.body()))
    }
}

impl<BodyType, Extractor> RunnerInput<Extractor> for Request<BodyType>
where
    Extractor: BodyDeserializer<Item = BodyType>,
    BodyType: DeserializeOwned,
{
    fn try_into(input: InternalResult<Request<String>>) -> InternalResult<Self>
    where
        Self: std::marker::Sized,
    {
        input.and_then(|input| input.and_then(|body| Extractor::deserialize(&body)))
    }
}

impl<BodyType, Extractor, RInput> RunnerInput<Extractor> for crate::result::Result<RInput>
where
    Extractor: BodyDeserializer<Item = BodyType>,
    BodyType: DeserializeOwned,
    RInput: RunnerInput<Extractor>,
{
    fn try_into(input: InternalResult<Request<String>>) -> InternalResult<Self>
    where
        Self: std::marker::Sized,
    {
        Ok(RInput::try_into(input).into())
    }
}

pub trait BodyDeserializer {
    type Item: DeserializeOwned;

    fn deserialize(content: &StandardBodyType) -> InternalResult<Self::Item>
    where
        Self: std::marker::Sized;
}
