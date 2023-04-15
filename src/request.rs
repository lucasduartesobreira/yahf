use crate::handler::Result;

pub type Method = http::Method;
pub type Uri = http::Uri;
pub type HttpRequest<T> = http::Request<T>;
pub type HttpBuilder = http::request::Builder;

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
        callback: impl FnOnce(T) -> Result<BodyType>,
    ) -> Result<Request<BodyType>> {
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
}

pub struct Builder {
    pub builder: HttpBuilder,
}

impl Builder {
    pub fn uri<T>(self, uri: T) -> Builder
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
            request: self.builder.body(body).unwrap(),
        }
    }

    pub fn method(self, method: Method) -> Self {
        Self {
            builder: self.builder.method(method),
        }
    }
}
