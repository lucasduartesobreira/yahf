use async_std::io::BufReader;
use futures::{AsyncBufReadExt, AsyncRead, AsyncReadExt};

use crate::handler::InternalResult;

pub type Method = http::Method;
pub type Uri = http::Uri;
pub type HttpRequest<T> = http::Request<T>;
pub type HttpBuilder = http::request::Builder;
pub type HttpHeaderName = http::HeaderName;
pub type HttpHeaderValue = http::HeaderValue;
pub type HttpHeaderMap<HeaderValue> = http::HeaderMap<HeaderValue>;

pub struct Request<T> {
    request: HttpRequest<T>,
}

impl Request<()> {
    pub fn builder() -> Builder {
        Builder {
            builder: HttpBuilder::new(),
        }
    }
}

const BAD_REQUEST: &str = "400 Bad Request";
const HTTP_VERSION_NOT_SUPPORTED: &str = "505 HTTP Version Not Supported";

impl<T> Request<T> {
    pub fn new(value: T) -> Self {
        Self {
            request: HttpRequest::new(value),
        }
    }

    pub fn body(&self) -> &T {
        self.request.body()
    }

    // TODO: Valuate if this will keep this fn or move to an from_parts style
    pub fn and_then<BodyType>(
        self,
        callback: impl FnOnce(T) -> InternalResult<BodyType>,
    ) -> InternalResult<Request<BodyType>> {
        let (parts, body) = self.request.into_parts();
        callback(body).map(|body| Request {
            request: HttpRequest::from_parts(parts, body),
        })
    }

    pub fn method(&self) -> &Method {
        self.request.method()
    }

    pub fn method_mut(&mut self) -> &mut Method {
        self.request.method_mut()
    }

    pub fn uri(&self) -> &Uri {
        self.request.uri()
    }

    pub fn uri_mut(&mut self) -> &mut Uri {
        self.request.uri_mut()
    }

    pub fn headers(&self) -> &HttpHeaderMap<HttpHeaderValue> {
        self.request.headers()
    }
}

impl Request<()> {
    #[inline]
    pub async fn from_stream(
        mut stream: &mut (impl AsyncRead + Unpin),
    ) -> std::result::Result<Request<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut buf_reader = BufReader::with_capacity(1024, &mut stream);
        let mut first = String::with_capacity(256);
        buf_reader
            .read_line(&mut first)
            .await
            .map_err(|_| BAD_REQUEST)?;

        let mut request_builder = Request::builder();

        first.pop();
        first.pop();
        let fl = first;

        let mut splitted_fl = fl.split(' ');
        let method = match splitted_fl.next() {
            Some(mtd) => Method::try_from(mtd).map_err(|_| BAD_REQUEST)?,
            None => Err(BAD_REQUEST)?,
        };

        let method = match method {
            Method::GET => method,
            Method::PUT => method,
            Method::POST => method,
            Method::DELETE => method,
            Method::OPTIONS => method,
            Method::HEAD => method,
            Method::TRACE => method,
            Method::PATCH => method,
            Method::CONNECT => method,
            _ => Err(BAD_REQUEST)?,
        };

        let uri = match splitted_fl.next() {
            Some(mtd) => Uri::try_from(mtd).map_err(|_| BAD_REQUEST)?,
            None => Err(BAD_REQUEST)?,
        };

        match splitted_fl.next() {
            Some("HTTP/1.1") => (),
            _ => Err(HTTP_VERSION_NOT_SUPPORTED)?,
        };

        let mut content_length = 0usize;

        let mut line = fl;
        line.clear();
        loop {
            buf_reader
                .read_line(&mut line)
                .await
                .map_err(|_| BAD_REQUEST)?;

            line.pop();
            line.pop();

            if line.is_empty() {
                break;
            }

            let splitted_header = line.split_once(':');
            match splitted_header {
                Some((header, value)) if http::header::CONTENT_LENGTH == header => {
                    request_builder = request_builder.header("Content-Length", value);
                    content_length = value
                        .trim()
                        .parse::<usize>()
                        .unwrap();
                }
                Some((header, value)) => {
                    match (
                        HttpHeaderName::try_from(header.trim()),
                        HttpHeaderValue::try_from(value.trim()),
                    ) {
                        (Ok(header), Ok(value)) => {
                            request_builder = request_builder.header(header, value);
                        }
                        _ => Err("400 Bad Request")?,
                    }
                }
                None => Err("400 Bad Request")?,
            }

            line.clear()
        }

        let mut body_string = vec![0u8; content_length];
        buf_reader
            .read_exact(&mut body_string)
            .await
            .map_err(|_| BAD_REQUEST)?;

        Ok(request_builder
            .uri(uri)
            .method(method)
            .body(String::from_utf8(body_string)?))
    }
}

pub struct Builder {
    pub builder: HttpBuilder,
}

impl Builder {
    pub fn uri<T>(self, uri: T) -> Self
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<http::Error>,
    {
        Self {
            builder: self.builder.uri(uri),
        }
    }

    pub fn body<T>(self, body: T) -> Request<T> {
        Request {
            request: self
                .builder
                .body(body)
                .unwrap(),
        }
    }

    pub fn method(self, method: Method) -> Self {
        Self {
            builder: self.builder.method(method),
        }
    }

    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HttpHeaderName: TryFrom<K>,
        <HttpHeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HttpHeaderValue: TryFrom<V>,
        <HttpHeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self {
            builder: self
                .builder
                .header(key, value),
        }
    }
}
