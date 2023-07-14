use crate::response::Response;

#[derive(Debug)]
pub struct Error {
    body: String,
    code: u16,
}

impl Error {
    pub fn new(body: String, code: u16) -> Self {
        Self { body, code }
    }

    pub fn body(&self) -> &String {
        &self.body
    }

    pub fn code(&self) -> &u16 {
        &self.code
    }
}

impl From<Error> for Response<String> {
    fn from(val: Error) -> Self {
        Response::builder()
            .status(val.code)
            .body(val.body)
    }
}
