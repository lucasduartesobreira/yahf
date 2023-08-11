use std::ops::{Deref, DerefMut};

use crate::handler::InternalResult;

pub type Method = http::Method;
pub type Uri = http::Uri;
pub type HttpRequest<T> = http::Request<T>;
pub type HttpBuilder = http::request::Builder;
pub type HttpHeaderName = http::HeaderName;
pub type HttpHeaderValue = http::HeaderValue;
pub type HttpHeaderMap<HeaderValue> = http::HeaderMap<HeaderValue>;

#[derive(Default)]
pub struct Request<T>(HttpRequest<T>);

impl<T> Deref for Request<T> {
    type Target = http::Request<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Request<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Request<()> {
    #[allow(dead_code)]
    pub(crate) fn builder() -> Builder {
        Builder {
            builder: HttpBuilder::new(),
        }
    }
}

impl<T> Request<T> {
    pub fn new(value: T) -> Self {
        Self(HttpRequest::new(value))
    }

    // TODO: Valuate if this will keep this fn or move to an from_parts style
    pub fn and_then<BodyType>(
        self,
        callback: impl FnOnce(T) -> InternalResult<BodyType>,
    ) -> InternalResult<Request<BodyType>> {
        let (parts, body) = self.0.into_parts();
        callback(body).map(|body| Request(HttpRequest::from_parts(parts, body)))
    }

    pub fn into_inner(self) -> HttpRequest<T> {
        self.0
    }
}

pub(crate) struct Builder {
    pub builder: HttpBuilder,
}

#[allow(dead_code)]
impl Builder {
    pub(crate) fn uri<T>(self, uri: T) -> Self
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<http::Error>,
    {
        Self {
            builder: self.builder.uri(uri),
        }
    }

    pub(crate) fn body<T>(self, body: T) -> Request<T> {
        Request(
            self.builder
                .body(body)
                .unwrap(),
        )
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

impl<T> From<Request<T>> for http::Request<T> {
    fn from(value: Request<T>) -> Self {
        value.0
    }
}

impl<T> From<http::Request<T>> for Request<T> {
    fn from(value: http::Request<T>) -> Self {
        Request(value)
    }
}
