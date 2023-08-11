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

impl From<http::Error> for Error {
    fn from(value: http::Error) -> Self {
        Self::new(value.to_string(), 500)
    }
}

impl From<Error> for Response<String> {
    fn from(val: Error) -> Self {
        Response::builder()
            .status(val.code)
            .body(val.body)
            .map_or_else(
                |err| {
                    http::Response::builder()
                        .status(500)
                        .body(err.to_string())
                        .expect("Error creating the error")
                },
                |res| res,
            )
            .into()
    }
}
