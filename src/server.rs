use futures::StreamExt;
use std::{
    convert::Infallible,
    future::ready,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    handler::{InternalResult, Runner},
    middleware::{AfterMiddleware, PreMiddleware},
    request::{self, Request},
    response::Response,
    router::Router,
};

use futures::Future;
use http::StatusCode;
use hyper::{
    server::{
        accept,
        conn::{AddrIncoming, AddrStream},
    },
    service::{make_service_fn, service_fn},
};

use request::Method;
use tls_listener::{AsyncTls, TlsListener};

pub struct Server<PreM, AfterM> {
    router: Router<PreM, AfterM>,
}

impl<PreM, FutP, ResultP, AfterM, FutA, ResultA> Deref for Server<PreM, AfterM>
where
    PreM: PreMiddleware<FutCallResponse = FutP>,
    FutP: Future<Output = ResultP>,
    ResultP: Into<InternalResult<Request<String>>>,
    AfterM: AfterMiddleware<FutCallResponse = FutA>,
    FutA: Future<Output = ResultA>,
    ResultA: Into<InternalResult<Response<String>>>,
{
    type Target = Router<PreM, AfterM>;

    fn deref(&self) -> &Self::Target {
        &self.router
    }
}

impl<PreM, FutP, ResultP, AfterM, FutA, ResultA> DerefMut for Server<PreM, AfterM>
where
    PreM: PreMiddleware<FutCallResponse = FutP>,
    FutP: Future<Output = ResultP>,
    ResultP: Into<InternalResult<Request<String>>>,
    AfterM: AfterMiddleware<FutCallResponse = FutA>,
    FutA: Future<Output = ResultA>,
    ResultA: Into<InternalResult<Response<String>>>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.router
    }
}

impl Server<(), ()> {
    pub fn new() -> Server<
        impl PreMiddleware<
            FutCallResponse = impl Future<Output = impl Into<InternalResult<Request<String>>>>,
        >,
        impl AfterMiddleware<
            FutCallResponse = impl Future<Output = impl Into<InternalResult<Response<String>>>>,
        >,
    > {
        Server {
            router: Router::new(),
        }
    }
}

macro_rules! method_reroute {
    ($method: ident) => {
        pub fn $method<FnIn, FnOut, Deserializer, Serializer, R>(
            mut self,
            path: &'static str,
            handler: R,
            deserializer: &Deserializer,
            serializer: &Serializer,
        ) -> Self
        where
            R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
            FnIn: 'static,
            FnOut: 'static,
            Deserializer: 'static,
            Serializer: 'static,
        {
            let router = self.router;
            let router = router.$method(path, handler, deserializer, serializer);
            self.router = router;
            self
        }
    };
}

impl<PreM, FutP, ResultP, AfterM, FutA, ResultA> Server<PreM, AfterM>
where
    PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
    FutP: Future<Output = ResultP> + std::marker::Send + 'static,
    ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
    AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
    FutA: Future<Output = ResultA> + std::marker::Send + 'static,
    ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
{
    method_reroute!(get);
    method_reroute!(put);
    method_reroute!(delete);
    method_reroute!(post);
    method_reroute!(trace);
    method_reroute!(options);
    method_reroute!(connect);
    method_reroute!(patch);
    method_reroute!(head);
    method_reroute!(all);

    pub fn method<FnIn, FnOut, Deserializer, Serializer, R>(
        mut self,
        method: Method,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) -> Self
    where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        let router = self.router;
        let router = router.method(method, path, handler, deserializer, serializer);
        self.router = router;
        self
    }

    pub fn router<OtherPreM, OtherAfterM, OtherFutA, OtherFutP, OtherResultP, OtherResultA>(
        self,
        router: Router<OtherPreM, OtherAfterM>,
    ) -> Self
    where
        OtherPreM: PreMiddleware<FutCallResponse = OtherFutP> + 'static,
        OtherAfterM: AfterMiddleware<FutCallResponse = OtherFutA> + 'static,
        OtherFutP: Future<Output = OtherResultP> + Send,
        OtherFutA: Future<Output = OtherResultA> + Send,
        OtherResultP: Into<InternalResult<Request<String>>> + Send,
        OtherResultA: Into<InternalResult<Response<String>>> + Send,
    {
        let new_router = self.router.router(router);
        Self { router: new_router }
    }

    pub fn pre<NewPreM, NewFut, NewResultP>(
        self,
        middleware: NewPreM,
    ) -> Server<impl PreMiddleware<FutCallResponse = impl Future<Output = NewResultP>>, AfterM>
    where
        NewPreM: PreMiddleware<FutCallResponse = NewFut>,
        NewFut: Future<Output = NewResultP>,
        NewResultP: Into<InternalResult<Request<String>>>,
    {
        let new_router = self.router.pre(middleware);

        Server { router: new_router }
    }

    pub fn after<NewAfterM, NewFut, NewResultA>(
        self,
        middleware: NewAfterM,
    ) -> Server<PreM, impl AfterMiddleware<FutCallResponse = impl Future<Output = NewResultA>>>
    where
        NewAfterM: AfterMiddleware<FutCallResponse = NewFut>,
        NewFut: Future<Output = NewResultA>,
        NewResultA: Into<InternalResult<Response<String>>>,
    {
        let new_router = self.router.after(middleware);

        Server { router: new_router }
    }

    pub async fn listen(self, addr: std::net::SocketAddr) -> Result<(), hyper::Error> {
        let server = Arc::new(self);
        let make_svc = make_service_fn(move |_: &AddrStream| {
            let server = server.clone();
            let service = service_fn(move |req| handle_req(server.clone(), req));
            async move { Ok::<_, Infallible>(service) }
        });

        let server = hyper::Server::bind(&addr).serve(make_svc);
        server.await?;
        Ok(())
    }

    pub async fn listen_tls<T: AsyncTls<C> + AsyncTls<AddrStream>, C: AsyncRead + AsyncWrite>(
        self,
        addr: std::net::SocketAddr,
        tls: T,
    ) -> Result<(), hyper::Error>
    where
        <T as AsyncTls<AddrStream>>::Error: Send + Sync + 'static,
        <T as AsyncTls<AddrStream>>::Stream: Send + Unpin + AsyncWrite + AsyncRead + 'static,
    {
        let server = Arc::new(self);
        let make_svc = make_service_fn(move |_| {
            let server = server.clone();
            let service = service_fn(move |req| handle_req(server.clone(), req));
            async move { Ok::<_, Infallible>(service) }
        });

        let listener = TlsListener::new(tls, AddrIncoming::bind(&addr)?).filter(|conn| {
            if let Err(err) = conn {
                eprintln!("Error: {:?}", err);
                ready(false)
            } else {
                ready(true)
            }
        });

        let server = hyper::Server::builder(accept::from_stream(listener)).serve(make_svc);
        server.await?;
        Ok(())
    }
}

async fn handle_req<PreM, FutP, ResultP, AfterM, FutA, ResultA>(
    server: Arc<Server<PreM, AfterM>>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, Box<dyn std::error::Error + Send + Sync>>
where
    PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
    FutP: Future<Output = ResultP> + std::marker::Send + 'static,
    ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
    AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
    FutA: Future<Output = ResultA> + std::marker::Send + 'static,
    ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
{
    let handler = server.find_route(req.method(), req.uri().path());

    let handler = match handler {
        Some(handler) => handler,
        None => {
            return Ok(hyper::Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(hyper::Body::empty())
                .unwrap());
        }
    };

    let (parts, body) = req.into_parts();
    let str = String::from_utf8(
        hyper::body::to_bytes(body)
            .await?
            .to_vec(),
    )?;
    let req_new = hyper::Request::from_parts(parts, str);

    let (parts, body) = handler
        .call(Ok(Request::from(req_new)))
        .await
        .map_or_else(|err| err.into(), |res| res)
        .into_inner()
        .into_parts();

    let body = hyper::Body::from(body);

    Ok(hyper::Response::from_parts(parts, body))
}

#[cfg(test)]
mod test {

    use std::net::SocketAddr;

    use futures::Future;
    use hyper::{Body, Client};

    use crate::{
        error::Error,
        handler::InternalResult,
        middleware::{AfterMiddleware, PreMiddleware},
        request::{Method, Request},
        response::Response,
        server::Server,
    };

    struct TestReq {
        req: hyper::Request<hyper::Body>,
        res: hyper::Response<&'static str>,
    }

    async fn run_req<PreM, FutP, ResultP, AfterM, FutA, ResultA>(
        server: Server<PreM, AfterM>,
        addr: SocketAddr,
        test_req: TestReq,
    ) -> Result<(), hyper::Error>
    where
        PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
        FutP: Future<Output = ResultP> + std::marker::Send + 'static,
        ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
        AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
        FutA: Future<Output = ResultA> + std::marker::Send + 'static,
        ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
    {
        tokio::spawn(server.listen(addr));

        let TestReq {
            mut req,
            res: expected_res,
        } = test_req;

        *req.uri_mut() = format!("http://localhost:{}/", addr.port())
            .parse()
            .unwrap();

        let client = Client::new();
        let response = client.request(req).await?;

        assert!(response.status() == expected_res.status());

        let body_str = String::from_utf8(
            hyper::body::to_bytes(response.into_body())
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();

        assert!(body_str.as_str() == expected_res.into_body());

        Ok(())
    }

    macro_rules! test_with_server {
        ($name: ident, $server: expr, $ip: literal, $req: expr, $res: expr) => {
            #[tokio::test]
            async fn $name() {
                let server = $server;
                let response = run_req(
                    server,
                    $ip.parse().unwrap(),
                    TestReq {
                        req: $req,
                        res: $res,
                    },
                )
                .await;

                assert!(response.is_ok(), "{:?}", response);
            }
        };
    }

    macro_rules! test_server_method {
        ($name: ident, $method: ident, $req: expr, $ip: literal) => {
            #[tokio::test]
            async fn $name() {
                let server = Server::new().$method(
                    "/",
                    || async { String::from("Hello world!") },
                    &(),
                    &String::with_capacity(0),
                );
                let response = run_req(
                    server,
                    $ip.parse().unwrap(),
                    TestReq {
                        req: $req,
                        res: hyper::Response::new("Hello world!"),
                    },
                )
                .await;

                assert!(response.is_ok(), "{:?}", response);
            }
        };
    }

    test_server_method!(
        test_server_get,
        get,
        hyper::Request::builder()
            .method(Method::GET)
            .body(Body::from(""))
            .unwrap(),
        "127.0.0.1:8000"
    );
    test_server_method!(
        test_server_post,
        post,
        hyper::Request::builder()
            .method(Method::POST)
            .body(Body::from(""))
            .unwrap(),
        "127.0.0.1:8001"
    );
    test_server_method!(
        test_server_put,
        put,
        hyper::Request::builder()
            .method(Method::PUT)
            .body(Body::from(""))
            .unwrap(),
        "127.0.0.1:8002"
    );
    test_server_method!(
        test_server_delete,
        delete,
        hyper::Request::builder()
            .method(Method::DELETE)
            .body(Body::from(""))
            .unwrap(),
        "127.0.0.1:8003"
    );
    test_server_method!(
        test_server_patch,
        patch,
        hyper::Request::builder()
            .method(Method::PATCH)
            .body(Body::from(""))
            .unwrap(),
        "127.0.0.1:8004"
    );

    #[tokio::test]
    async fn test_server_head() {
        let server = Server::new().head(
            "/",
            || async { String::from("Hello world!") },
            &(),
            &String::with_capacity(0),
        );
        let response = run_req(
            server,
            "127.0.0.1:8005"
                .parse()
                .unwrap(),
            TestReq {
                req: hyper::Request::builder()
                    .method(Method::HEAD)
                    .body(Body::from(""))
                    .unwrap(),
                res: hyper::Response::new(""),
            },
        )
        .await;

        assert!(response.is_ok(), "{:?}", response);
    }
    test_server_method!(
        test_server_options,
        options,
        hyper::Request::builder()
            .method(Method::OPTIONS)
            .body(Body::from(""))
            .unwrap(),
        "127.0.0.1:8006"
    );
    test_server_method!(
        test_server_trace,
        trace,
        hyper::Request::builder()
            .method(Method::TRACE)
            .body(Body::from(""))
            .unwrap(),
        "127.0.0.1:8007"
    );
    test_server_method!(
        test_server_all,
        all,
        hyper::Request::builder()
            .method(Method::GET)
            .body(Body::from(""))
            .unwrap(),
        "127.0.0.1:8008"
    );
    test_with_server!(
        test_pre_error,
        Server::new()
            .pre(|_| async {
                crate::result::Result::from(Err(Error::new("PreMiddleware error".into(), 422)))
            })
            .get(
                "/",
                || async { "Hello world".to_owned() },
                &(),
                &String::with_capacity(0)
            ),
        "127.0.0.1:8009",
        hyper::Request::builder()
            .method(Method::GET)
            .body(Body::from(""))
            .unwrap(),
        hyper::Response::builder()
            .status(422)
            .body("PreMiddleware error")
            .unwrap()
    );

    test_with_server!(
        test_pre_error_handled,
        Server::new()
            .pre(|_| async {
                crate::result::Result::from(Err(Error::new("PreMiddleware error".into(), 422)))
            })
            .pre(|req: crate::result::Result<Request<String>>| async {
                crate::result::Result::from(req.into_inner().map_or_else(
                    |_| {
                        Ok(crate::request::Request::new(String::from(
                            "PreMiddleware fixed error",
                        )))
                    },
                    Ok,
                ))
            })
            .get(
                "/",
                || async { "Hello world".to_owned() },
                &(),
                &String::with_capacity(0)
            ),
        "127.0.0.1:8010",
        hyper::Request::builder()
            .method(Method::GET)
            .body(Body::from(""))
            .unwrap(),
        hyper::Response::builder()
            .status(200)
            .body("Hello world")
            .unwrap()
    );

    test_with_server!(
        test_after_error,
        Server::new()
            .after(|_| async {
                crate::result::Result::from(Err(Error::new("AfterMiddleware error".into(), 422)))
            })
            .get(
                "/",
                || async { "Hello world".to_owned() },
                &(),
                &String::with_capacity(0)
            ),
        "127.0.0.1:8011",
        hyper::Request::builder()
            .method(Method::GET)
            .body(Body::from(""))
            .unwrap(),
        hyper::Response::builder()
            .status(422)
            .body("AfterMiddleware error")
            .unwrap()
    );

    test_with_server!(
        test_after_error_handled,
        Server::new()
            .after(|_| async {
                crate::result::Result::from(Err(Error::new("AfterMiddleware error".into(), 422)))
            })
            .after(|res: crate::result::Result<Response<String>>| async {
                crate::result::Result::from(res.into_inner().map_or_else(
                    |_| {
                        Ok(crate::response::Response::new(
                            "AfterMiddleware Handled Error".to_owned(),
                        ))
                    },
                    Ok,
                ))
            })
            .get(
                "/",
                || async { "Hello world".to_owned() },
                &(),
                &String::with_capacity(0)
            ),
        "127.0.0.1:8012",
        hyper::Request::builder()
            .method(Method::GET)
            .body(Body::from(""))
            .unwrap(),
        hyper::Response::builder()
            .status(200)
            .body("AfterMiddleware Handled Error")
            .unwrap()
    );
}
