use http::{Request, Response};
use serde::{Deserialize, Serialize};

type HResponse<T> = Response<T>;

#[derive(Debug, Deserialize, Serialize)]
pub struct HttpResponse<ResBody> {
    pub body: Option<ResBody>,
}

#[derive(Debug)]
pub struct HttpError {
    pub _code: u32,
    pub _body: String,
}

#[derive(Debug)]
pub enum Error {
    ParseBody(String),
    RequestError(HttpError),
}
