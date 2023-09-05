use serde::de::DeserializeOwned;

use crate::{
    error::Error,
    handler::{Json, StandardBodyType},
    result::InternalResult,
};

pub trait BodyDeserializer {
    type Item: DeserializeOwned;

    fn deserialize(content: &StandardBodyType) -> InternalResult<Self::Item>
    where
        Self: std::marker::Sized;
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
