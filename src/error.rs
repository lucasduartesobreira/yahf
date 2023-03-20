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
