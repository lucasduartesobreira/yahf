use serde::Serialize;

use crate::{response::Response, result::InternalResult, serializer::BodySerializer};

pub trait RunnerOutput<Serializer> {
    fn try_into(self) -> InternalResult<Response<String>>;
}

impl<BodyType, Serializer> RunnerOutput<Serializer> for Response<BodyType>
where
    Serializer: BodySerializer<Item = BodyType>,
    BodyType: Serialize,
{
    fn try_into(self) -> InternalResult<Response<String>> {
        self.and_then(|body| Serializer::serialize(body))
    }
}

impl<BodyType, Serializer> RunnerOutput<Serializer> for BodyType
where
    Serializer: BodySerializer<Item = BodyType>,
    BodyType: Serialize,
{
    fn try_into(self) -> InternalResult<Response<String>> {
        Serializer::serialize(self).map(Response::new)
    }
}

impl<BodyType, Serializer, BasicRunnerOutput> RunnerOutput<Serializer>
    for crate::result::Result<BasicRunnerOutput>
where
    Serializer: BodySerializer<Item = BodyType>,
    BodyType: Serialize,
    BasicRunnerOutput: RunnerOutput<Serializer>,
{
    fn try_into(self) -> InternalResult<Response<String>> {
        self.into_inner()
            .and_then(|resp| BasicRunnerOutput::try_into(resp))
    }
}
