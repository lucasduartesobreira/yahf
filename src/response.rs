use std::ops::{Deref, DerefMut};

use crate::result::InternalResult;

pub use http::response::Builder as HttpResponseBuilder;
use http::response::Parts;
pub use http::Response as HttpResponse;

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
    pub fn builder() -> HttpResponseBuilder {
        HttpResponseBuilder::new()
    }
}

impl<T> Response<T> {
    pub fn new(value: T) -> Self {
        Self(HttpResponse::new(value))
    }

    // TODO: Valuate if this will keep this fn or move to an from_parts style
    pub fn and_then<BodyType>(
        self,
        callback: impl FnOnce(T) -> InternalResult<BodyType>,
    ) -> InternalResult<Response<BodyType>> {
        let (parts, body) = self.0.into_parts();
        callback(body).map(|body| Response(HttpResponse::from_parts(parts, body)))
    }

    pub fn into_inner(self) -> HttpResponse<T> {
        self.0
    }
    /// Consumes the response, returning just the body.
    ///
    /// Just a dummy to http::Request function
    #[inline]
    pub fn into_body(self) -> T {
        self.0.into_body()
    }
    /// Consumes the response returning the head and body parts.
    ///
    /// Just a dummy to http::Request function
    #[inline]
    pub fn into_parts(self) -> (Parts, T) {
        self.0.into_parts()
    }

    /// Consumes the response returning a new response with body mapped to the
    /// return type of the passed in function.
    ///
    /// Just a dummy to http::Request function
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
