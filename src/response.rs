//! NewType of [Response](http::Response)
use std::ops::{Deref, DerefMut};

use crate::result::InternalResult;

pub use http::response::Builder as HttpResponseBuilder;
use http::response::Parts;
pub use http::Response as HttpResponse;

/// NewType of [Response](http::Response)
#[derive(Debug)]
pub struct Response<T>(HttpResponse<T>);

impl<T> Deref for Response<T> {
    type Target = http::Response<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Response<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<http::Response<T>> for Response<T> {
    fn from(value: http::Response<T>) -> Self {
        Response(value)
    }
}

impl<T> From<Response<T>> for http::Response<T> {
    fn from(value: Response<T>) -> Self {
        value.0
    }
}

impl Response<()> {
    /// Create a new instance of [Builder](http::response::Builder)
    pub fn builder() -> HttpResponseBuilder {
        HttpResponseBuilder::new()
    }
}

impl<T> Response<T> {
    /// Create a new instance of [Response](crate::response::Response)
    pub fn new(value: T) -> Self {
        Self(HttpResponse::new(value))
    }

    /// Creates a new `Response` with the given head and body
    ///
    /// Just a dummy to [from_parts](http::Response::from_parts) function
    #[inline]
    pub fn from_parts(parts: Parts, body: T) -> Response<T> {
        Response(HttpResponse::from_parts(parts, body))
    }

    /// Transform the `Body` of a [Response](crate::response::Response) using a callback that
    /// returns a result
    pub fn and_then<BodyType>(
        self,
        callback: impl FnOnce(T) -> InternalResult<BodyType>,
    ) -> InternalResult<Response<BodyType>> {
        let (parts, body) = self.0.into_parts();
        callback(body).map(|body| Response(HttpResponse::from_parts(parts, body)))
    }

    /// Consume [NewType](crate::response::Response) and Return the original
    /// [Response](http::Response)
    pub fn into_inner(self) -> HttpResponse<T> {
        self.0
    }
    /// Consumes the response, returning just the body.
    ///
    /// Just a dummy to [into_body](http::Response::into_body) function
    #[inline]
    pub fn into_body(self) -> T {
        self.0.into_body()
    }
    /// Consumes the response returning the head and body parts.
    ///
    /// Just a dummy to [into_parts](http::Response::into_parts) function
    #[inline]
    pub fn into_parts(self) -> (Parts, T) {
        self.0.into_parts()
    }

    /// Consumes the response returning a new response with body mapped to the
    /// return type of the passed in function.
    ///
    /// Just a dummy to [map](http::Response::map) function
    #[inline]
    pub fn map<F, U>(self, f: F) -> Response<U>
    where
        F: FnOnce(T) -> U,
    {
        Response(self.0.map(f))
    }
}

impl From<Response<String>> for InternalResult<Response<String>> {
    fn from(val: Response<String>) -> Self {
        Ok(val)
    }
}
