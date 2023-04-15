use crate::handler::Result;

pub type HttpResponse<T> = http::Response<T>;

pub struct Response<T> {
    response: HttpResponse<T>,
}

impl<T> Response<T> {
    pub fn new(value: T) -> Self {
        Self {
            response: HttpResponse::new(value),
        }
    }

    pub fn body(&self) -> &T {
        self.response.body()
    }

    // TODO: Valuate if this will keep this fn or move to an from_parts style
    pub fn and_then<BodyType>(
        self,
        callback: impl FnOnce(T) -> Result<BodyType>,
    ) -> Result<Response<BodyType>> {
        let (parts, body) = self.response.into_parts();
        callback(body).map(|body| Response {
            response: HttpResponse::from_parts(parts, body),
        })
    }
}
