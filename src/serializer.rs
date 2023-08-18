use serde::Serialize;

use crate::{
    error::Error,
    handler::{InternalResult, Json, StandardBodyType},
};

pub trait BodySerializer {
    type Item;

    fn serialize(content: Self::Item) -> InternalResult<StandardBodyType>;
}

impl<T> BodySerializer for Json<T>
where
    T: Serialize,
{
    type Item = T;

    fn serialize(content: Self::Item) -> InternalResult<String> {
        serde_json::to_string(&content).map_err(|err| Error::new(err.to_string(), 422))
    }
}

impl BodySerializer for String {
    type Item = String;

    fn serialize(content: Self::Item) -> InternalResult<StandardBodyType> {
        Ok(content.to_owned())
    }
}
