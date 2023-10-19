//! Newtype of [Request](http::Request)
use std::ops::{Deref, DerefMut};

use crate::result::InternalResult;

pub use http::request::Builder as HttpBuilder;
pub use http::request::Parts;
pub use http::HeaderName;
pub use http::HeaderValue;
pub use http::Method;
pub use http::Request as HttpRequest;
pub use http::Uri;

/// Newtype of [Request](http::Request)
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
    /// Create a new Request
    pub fn new(value: T) -> Self {
        Self(HttpRequest::new(value))
    }

    /// Creates a new `Request` with the given components parts and body.
    ///
    /// Just a dummy to [from_parts](http::Request::from_parts) function
    #[inline]
    pub fn from_parts(parts: Parts, body: T) -> Request<T> {
        Request(HttpRequest::from_parts(parts, body))
    }

    // TODO: Valuate if this will keep this fn or move to an from_parts style
    pub(crate) fn and_then<BodyType>(
        self,
        callback: impl FnOnce(T) -> InternalResult<BodyType>,
    ) -> InternalResult<Request<BodyType>> {
        let (parts, body) = self.0.into_parts();
        callback(body).map(|body| Request(HttpRequest::from_parts(parts, body)))
    }

    /// Consume [NewType](crate::request::Request) and Return the original [Request](http::Request)
    pub fn into_inner(self) -> HttpRequest<T> {
        self.0
    }

    /// Consumes the request, returning just the body.
    ///
    /// Just a dummy to [into_body](http::Request::into_body) function
    #[inline]
    pub fn into_body(self) -> T {
        self.0.into_body()
    }

    /// Consumes the request returning the head and body parts.
    ///
    /// Just a dummy to [into_parts](http::Request::into_parts) function
    #[inline]
    pub fn into_parts(self) -> (Parts, T) {
        self.0.into_parts()
    }

    /// Consumes the request returning a new request with body mapped to the
    /// return type of the passed in function.
    ///
    /// Just a dummy to [map](http::Request::map) function
    #[inline]
    pub fn map<F, U>(self, f: F) -> Request<U>
    where
        F: FnOnce(T) -> U,
    {
        Request(self.0.map(f))
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
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
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

impl From<Request<String>> for InternalResult<Request<String>> {
    fn from(val: Request<String>) -> Self {
        Ok(val)
    }
}
