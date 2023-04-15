use crate::response::Response;

#[derive(Debug)]
pub struct Error {
    body: String,
}

impl Error {
    pub fn new(body: String) -> Self {
        Self { body }
    }
}

impl From<Error> for Response<String> {
    fn from(val: Error) -> Self {
        Response::new(val.body)
    }
}
