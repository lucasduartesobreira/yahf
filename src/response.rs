use crate::handler::Result;

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
    pub fn and_then<BodyType>(
        self,
        callback: impl FnOnce(T) -> Result<BodyType>,
    ) -> Result<Response<BodyType>> {
        let body = self.body;
        callback(body).map(Response::<BodyType>::new)
    }
}
