use crate::handler::InternalResult;

pub type Method = http::Method;
pub type Uri = http::Uri;
pub type HttpRequest<T> = http::Request<T>;
pub type HttpBuilder = http::request::Builder;
pub type HttpHeaderName = http::HeaderName;
pub type HttpHeaderValue = http::HeaderValue;
pub type HttpHeaderMap<HeaderValue> = http::HeaderMap<HeaderValue>;

pub struct Request<T>(HttpRequest<T>);

impl Request<()> {
    pub fn builder() -> Builder {
        Builder {
            builder: HttpBuilder::new(),
        }
    }
}

impl<T> Request<T> {
    pub fn new(value: T) -> Self {
        Self(HttpRequest::new(value))
    }

    pub fn body(&self) -> &T {
        self.0.body()
    }

    // TODO: Valuate if this will keep this fn or move to an from_parts style
    pub fn and_then<BodyType>(
        self,
        callback: impl FnOnce(T) -> InternalResult<BodyType>,
    ) -> InternalResult<Request<BodyType>> {
        let (parts, body) = self.0.into_parts();
        callback(body).map(|body| Request(HttpRequest::from_parts(parts, body)))
    }

    pub fn method(&self) -> &Method {
        self.0.method()
    }

    pub fn method_mut(&mut self) -> &mut Method {
        self.0.method_mut()
    }

    pub fn uri(&self) -> &Uri {
        self.0.uri()
    }

    pub fn uri_mut(&mut self) -> &mut Uri {
        self.0.uri_mut()
    }

    pub fn headers(&self) -> &HttpHeaderMap<HttpHeaderValue> {
        self.0.headers()
    }

    pub fn from_inner(req: HttpRequest<T>) -> Self {
        Self(req)
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
