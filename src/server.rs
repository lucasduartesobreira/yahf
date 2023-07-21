use std::{
    convert::Infallible,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    handler::{InternalResult, Runner},
    middleware::{AfterMiddleware, PreMiddleware},
    request::{self, Request},
    response::Response,
    router::Router,
};

use async_std::prelude::*;

use http::StatusCode;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
};

use request::Method;

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
}

async fn handle_req<PreM, FutP, ResultP, AfterM, FutA, ResultA>(
    server: Arc<Server<PreM, AfterM>>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, Infallible>
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
    /*
     *let str = body
     *    .try_fold(String::with_capacity(1024), |mut acc, ch| async {
     *        ch.lines()
     *            .try_fold(acc, |mut acc, l| async move {
     *                acc.push_str(l.as_str());
     *                Ok(acc)
     *            })
     *            .await
     *        //acc.push(ch.lines().co);
     *    })
     *    .await;
     */
    let str = String::from_utf8(
        hyper::body::to_bytes(body)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let req_new = hyper::Request::from_parts(parts, str);

    let (parts, body) = handler
        .call(Ok(Request::from_inner(req_new)))
        .await
        .map_or_else(|err| err.into(), |res| res)
        .into_inner()
        .into_parts();

    let body = hyper::Body::from(body);

    Ok(hyper::Response::from_parts(parts, body))
}
