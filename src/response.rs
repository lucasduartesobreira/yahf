use http::{HeaderMap, HeaderValue};

use crate::handler::InternalResult;

pub type HttpResponse<T> = http::Response<T>;
type HttpResponseBuilder = http::response::Builder;
type StatusCode = http::StatusCode;

pub struct ResponseBuilder {
    builder: HttpResponseBuilder,
}

impl ResponseBuilder {
    pub fn status<T>(self, status: T) -> ResponseBuilder
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    {
        ResponseBuilder {
            builder: self.builder.status(status),
        }
    }

    pub fn body<T>(self, body: T) -> Response<T> {
        Response {
            response: self
                .builder
                .body(body)
                .unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Response<T> {
    response: HttpResponse<T>,
}

impl Response<()> {
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder {
            builder: HttpResponseBuilder::new(),
        }
    }
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
        callback: impl FnOnce(T) -> InternalResult<BodyType>,
    ) -> InternalResult<Response<BodyType>> {
        let (parts, body) = self.response.into_parts();
        callback(body).map(|body| Response {
            response: HttpResponse::from_parts(parts, body),
        })
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.response.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        self.response.headers_mut()
    }
}

impl Response<String> {
    #[inline]
    pub fn format(self) -> String {
        let mut response = self;
        let content_size = response.body().len();
        response
            .headers_mut()
            .append("Content-Length", content_size.into());

        let status = response.status();

        let response_string = format!(
            "HTTP/1.1 {} {}\r\n{}\r\n{}",
            status.as_u16(),
            status
                .canonical_reason()
                .unwrap(),
            response
                .headers()
                .into_iter()
                .fold(String::with_capacity(1024), |mut acc, (name, value)| {
                    acc.push_str(format!("{}:{}\r\n", name, value.to_str().unwrap()).as_str());
                    acc
                }),
            response.body()
        );

        response_string
    }

    pub fn into_inner(self) -> HttpResponse<String> {
        self.response
    }
}
