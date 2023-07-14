use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    handler::{InternalResult, Runner},
    middleware::{AfterMiddleware, PreMiddleware},
    request::{self, HttpHeaderName, HttpHeaderValue, Request, Uri},
    response::Response,
    router::Router,
};
use async_std::io::BufReader;
use async_std::net::{TcpListener, ToSocketAddrs};
use async_std::prelude::*;
use async_std::task;

use futures::{AsyncRead, AsyncWrite, StreamExt};
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

    pub fn listen<A: ToSocketAddrs + Display>(self, addr: A) -> ListenResult<()> {
        task::block_on(accept_loop(self, addr))
    }
}
type ListenResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn accept_loop<PreM, FutP, ResultP, AfterM, FutA, ResultA>(
    server: Server<PreM, AfterM>,
    addr: impl ToSocketAddrs + Display,
) -> ListenResult<()>
where
    PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
    FutP: Future<Output = ResultP> + std::marker::Send + 'static,
    ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
    AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
    FutA: Future<Output = ResultA> + std::marker::Send + 'static,
    ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
{
    let server = Arc::new(server);
    let listener = TcpListener::bind(addr)
        .await
        .unwrap();
    println!("Start listening on {}", listener.local_addr().unwrap());
    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        handle_stream(server.clone(), stream);
    }
    Ok(())
}

fn handle_stream<PreM, FutP, ResultP, AfterM, FutA, ResultA>(
    server: Arc<Server<PreM, AfterM>>,
    mut stream: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
) -> async_std::task::JoinHandle<()>
where
    PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
    FutP: Future<Output = ResultP> + std::marker::Send + 'static,
    ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
    AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
    FutA: Future<Output = ResultA> + std::marker::Send + 'static,
    ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
{
    task::spawn(async move {
        let fut = connection_loop(server, &mut stream);
        let response = match fut.await {
            Ok(resp) => resp,
            Err(err) => {
                format!("HTTP/1.1 {}\r\n\r\n", err)
            }
        };

        stream
            .write_all(response.as_bytes())
            .await;
        stream.flush().await;
    })
}

const BAD_REQUEST: &str = "400 Bad Request";
const NOT_FOUND: &str = "404 Not Found";
const HTTP_VERSION_NOT_SUPPORTED: &str = "505 HTTP Version Not Supported";

async fn connection_loop<PreM, FutP, ResultP, AfterM, FutA, ResultA>(
    server: Arc<Server<PreM, AfterM>>,
    mut stream: &mut (impl AsyncRead + Unpin),
) -> ListenResult<String>
where
    PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
    FutP: Future<Output = ResultP> + std::marker::Send + 'static,
    ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
    AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
    FutA: Future<Output = ResultA> + std::marker::Send + 'static,
    ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
{
    let mut buf_reader = BufReader::new(&mut stream);
    let mut first = String::with_capacity(1024);
    buf_reader
        .read_line(&mut first)
        .await
        .map_err(|_| BAD_REQUEST)?;

    let request_builder = Request::builder();

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

    let handler = server.find_route(&method, &uri.to_string());
    let handler = match handler {
        Some(handler) => handler,
        None => Err(NOT_FOUND)?,
    };

    let mut request_builder = request_builder
        .method(method)
        .uri(uri);
    let mut content_length = 0usize;

    loop {
        let mut line = String::with_capacity(100);
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
    }

    let mut body_string = vec![0u8; content_length];
    buf_reader
        .read_exact(&mut body_string)
        .await
        .map_err(|_| BAD_REQUEST)?;

    let body_string = String::from_utf8(body_string)?;

    let request = request_builder.body(body_string);

    let response = handler
        .call(request.into())
        .await
        .map_or_else(|err| err.into(), |resp| resp);

    let response_string = format!(
        "HTTP/1.1 {} {}\r\n{}\r\n{}",
        response.status().as_u16(),
        response
            .status()
            .canonical_reason()
            .unwrap(),
        response
            .headers()
            .into_iter()
            .fold(String::new(), |mut acc, (name, value)| {
                acc.push_str(format!("{}:{}\r\n", name, value.to_str().unwrap()).as_str());
                acc
            }),
        response.body()
    );

    Ok(response_string)
}

mod test_utils {
    use std::cmp::min;
    use std::pin::Pin;

    use futures::io::Error;
    use futures::task::{Context, Poll};
    use futures::{AsyncRead, AsyncWrite};

    #[derive(Clone)]
    pub struct MockTcpStream {
        pub read_data: Vec<u8>,
        pub write_data: Vec<u8>,
    }

    impl AsyncRead for MockTcpStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _: &mut Context,
            buf: &mut [u8],
        ) -> Poll<Result<usize, Error>> {
            let size: usize = min(self.read_data.len(), buf.len());
            buf[..size].copy_from_slice(&self.read_data[..size]);
            self.read_data = self
                .read_data
                .drain(size..)
                .collect::<Vec<_>>();
            Poll::Ready(Ok(size))
        }
    }

    impl AsyncWrite for MockTcpStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _: &mut Context,
            buf: &[u8],
        ) -> Poll<Result<usize, Error>> {
            let mut a = buf.to_vec();
            self.write_data.append(&mut a);

            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
    }

    impl Unpin for MockTcpStream {}
}

#[cfg(test)]
mod test_server_routing {

    use async_std_test::async_test;
    use futures::Future;
    use serde::{Deserialize, Serialize};

    use crate::{
        handler::{GenericResponse, InternalResult, Json},
        middleware::{AfterMiddleware, PreMiddleware},
        request::{Method, Request},
        response::Response,
    };

    use super::Server;

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct TestStruct {
        correct: bool,
    }

    async fn test_handler_with_req_and_res(_req: Request<TestStruct>) -> GenericResponse {
        Response::new(serde_json::to_string(&TestStruct { correct: true }).unwrap())
    }

    async fn run_test<PreM, FutP, ResultP, AfterM, FutA, ResultA>(
        server: &Server<PreM, AfterM>,
        req: Request<String>,
    ) -> GenericResponse
    where
        PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
        FutP: Future<Output = ResultP> + std::marker::Send + 'static,
        ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
        AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
        FutA: Future<Output = ResultA> + std::marker::Send + 'static,
        ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
    {
        let method = req.method();

        let path = req.uri().path().to_string();

        let handler = server.find_route(method, path.as_str());

        assert!(handler.is_some());

        handler
            .unwrap()
            .call(req.into())
            .await
            .unwrap()
    }

    #[async_test]
    async fn test_handler_receiving_req_and_res() -> std::io::Result<()> {
        let server = Server::new();

        let server = server.method(
            Method::GET,
            "/aaaa/bbbb",
            test_handler_with_req_and_res,
            &Json::new(),
            &String::from(""),
        );

        let request = Request::builder()
            .uri("/aaaa/bbbb")
            .body(serde_json::json!({ "correct": false }).to_string());

        let response = run_test(&server, request).await;

        assert_eq!(
            response.body(),
            &serde_json::json!({ "correct": true }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_server_fn_all() -> std::io::Result<()> {
        let server = Server::new();

        let server = server.all(
            "/test/all",
            test_handler_with_req_and_res,
            &Json::new(),
            &String::new(),
        );

        let req_body = serde_json::json!({ "correct": false }).to_string();
        let expected_res_body = serde_json::json!({ "correct": true }).to_string();

        let request = Request::builder()
            .uri("/test/all")
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(Method::POST)
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(Method::PUT)
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(Method::DELETE)
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(Method::TRACE)
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(Method::OPTIONS)
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(Method::PATCH)
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(Method::CONNECT)
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(Method::HEAD)
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());

        Ok(())
    }

    macro_rules! test_method {
        ($sla:tt, $mtd:ident, $upper_mtd:ident) => {
    #[async_test]
    async fn $sla() -> std::io::Result<()> {
        let  server = Server::new();

        let server = server.$mtd(
            "/test/all",
            test_handler_with_req_and_res,
            &Json::new(),
            &String::new(),
        );

        let req_body = serde_json::json!({ "correct": false }).to_string();
        let expected_res_body = serde_json::json!({ "correct": true }).to_string();

        let request = Request::builder()
            .uri("/test/all")
            .method(Method::$upper_mtd)
            .body(req_body.clone());

        let response = run_test(&server, request).await;

        assert_eq!(*response.body(), expected_res_body.clone());
        Ok(())
    }
        };
    }

    test_method!(test_only_get, get, GET);
    test_method!(test_only_post, post, POST);
    test_method!(test_only_put, put, PUT);
    test_method!(test_only_delete, delete, DELETE);
    test_method!(test_only_trace, trace, TRACE);
    test_method!(test_only_option, options, OPTIONS);
    test_method!(test_only_connect, connect, CONNECT);
    test_method!(test_only_head, head, HEAD);
}

#[cfg(test)]
mod test_connection_loop {
    use std::sync::Arc;

    use async_std_test::async_test;
    use futures::Future;
    use serde::{Deserialize, Serialize};

    use crate::handler::InternalResult;
    use crate::middleware::{AfterMiddleware, PreMiddleware};
    use crate::request::Method;
    use crate::response::Response;
    use crate::server::connection_loop;
    use crate::server::test_utils::MockTcpStream;
    use crate::{handler::Json, request::Request, server::Server};

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct TestStruct {
        correct: bool,
    }

    async fn test_handler_with_req_and_res(_req: Request<TestStruct>) -> Response<TestStruct> {
        Response::new(TestStruct { correct: true })
    }

    async fn test_handler_unity_res() -> Response<TestStruct> {
        Response::new(TestStruct { correct: true })
    }

    async fn test_handler_req_body_and_res(_req: TestStruct) -> Response<TestStruct> {
        Response::new(TestStruct { correct: true })
    }

    async fn test_handler_with_req_and_res_body(_req: Request<TestStruct>) -> TestStruct {
        TestStruct { correct: true }
    }

    async fn test_handler_unity_res_body() -> TestStruct {
        TestStruct { correct: true }
    }

    async fn test_handler_req_body_and_res_body(_req: TestStruct) -> TestStruct {
        TestStruct { correct: true }
    }

    struct TestConfig {
        response: Response<String>,
        request: Request<String>,
    }

    async fn run_test<PreM, FutP, ResultP, AfterM, FutA, ResultA>(
        server: Arc<Server<PreM, AfterM>>,
        test_config: TestConfig,
    ) where
        PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
        FutP: Future<Output = ResultP> + std::marker::Send + 'static,
        ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
        AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
        FutA: Future<Output = ResultA> + std::marker::Send + 'static,
        ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
    {
        // TODO: Implement ToString for Request
        let request = test_config.request;
        let response = test_config.response;
        let method = request.method().to_string();
        let uri = request.uri().to_string();
        let ver = "HTTP/1.1";
        let header = request
            .headers()
            .iter()
            .fold(String::from("\r\n"), |acc, (key, value)| {
                format!("{}:{}\r\n{}", key, value.to_str().unwrap(), acc)
            });
        let body = request.body();
        let input_request = format!("{} {} {}\r\n{}{}", method, uri, ver, header, body);
        let mut stream = MockTcpStream {
            read_data: input_request.into_bytes(),
            write_data: vec![],
        };

        let response_a = connection_loop(server, &mut stream).await;

        // TODO: Implement ToString for Response
        let expected_contents = response.body();
        let expected_status_code = response.status().as_u16();
        let expected_status_message = response
            .status()
            .canonical_reason();
        let expected_response = format!(
            "HTTP/1.1 {} {}\r\n\r\n{}",
            expected_status_code,
            expected_status_message.unwrap(),
            expected_contents
        );

        assert!(response_a
            .map_or_else(|err| format!("HTTP/1.1 {}", err), |res| res)
            .starts_with(&expected_response));
    }

    macro_rules! test_connection_loop {
        ($test_name: tt, $fn: ident, $des: expr) => {
    #[async_test]
    async fn $test_name() -> std::io::Result<()> {
        let  server = Server::new();
        let server = server.all(
            "/aaaaa",
            $fn,
            &$des,
            &Json::default(),
        );
        let req_body = serde_json::json!({"correct": false}).to_string();
        let res_body = serde_json::json!({"correct": true}).to_string();

        let request = Request::builder()
            .uri("/aaaaa")
            .method(Method::GET)
            .header("Content-Length", req_body.len())
            .body(req_body);
        let response = Response::builder()
            .status(200)
            .body(res_body);
        let test_config = TestConfig { request, response };

        let server = Arc::new(server);

        run_test(server, test_config).await;

        Ok(())
    }

        };
    }

    test_connection_loop!(
        test_request_and_response,
        test_handler_with_req_and_res,
        Json::default()
    );
    test_connection_loop!(test_unity_response, test_handler_unity_res, ());
    test_connection_loop!(
        test_req_body_and_response,
        test_handler_req_body_and_res,
        Json::default()
    );
    test_connection_loop!(
        test_req_and_res_body,
        test_handler_with_req_and_res_body,
        Json::default()
    );
    test_connection_loop!(test_unity_res_body, test_handler_unity_res_body, ());
    test_connection_loop!(
        test_req_body_and_res_body,
        test_handler_req_body_and_res_body,
        Json::default()
    );

    async fn test_for_error<PreM, FutP, ResultP, AfterM, FutA, ResultA>(
        server: Arc<Server<PreM, AfterM>>,
        request: String,
        response: String,
    ) where
        PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
        FutP: Future<Output = ResultP> + std::marker::Send + 'static,
        ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
        AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
        FutA: Future<Output = ResultA> + std::marker::Send + 'static,
        ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
    {
        // TODO: Implement ToString for Request
        let mut stream = MockTcpStream {
            read_data: request.into_bytes(),
            write_data: vec![],
        };

        let error = connection_loop(server, &mut stream).await;

        // TODO: Implement ToString for Response
        println!("{:?}", error);
        match error {
            Ok(res) => assert!(res.starts_with(&response)),
            Err(err) => assert!(err
                .to_string()
                .starts_with(response.as_str())),
        }
    }

    macro_rules! test_error_in_connection_loop {
        ($test_name:ident, $fn: ident, $des: expr, $send:literal, $expected: literal) => {
            #[async_test]
            async fn $test_name() -> std::io::Result<()> {
                let server = Server::new();
                let server = server.all("/aaaaa", $fn, &$des, &Json::default());

                let server = Arc::new(server);

                let request = $send.to_owned();
                let response = $expected.to_owned();

                test_for_error(server.clone(), request, response).await;

                Ok(())
            }
        };
    }

    test_error_in_connection_loop!(
        test_no_payload_unity_res,
        test_handler_unity_res,
        (),
        "",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_no_payload_unity_res_body,
        test_handler_unity_res_body,
        (),
        "",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_no_payload_req_res,
        test_handler_with_req_and_res,
        Json::default(),
        "",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_no_payload_req_res_body,
        test_handler_with_req_and_res_body,
        Json::default(),
        "",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_no_payload_req_body_res,
        test_handler_req_body_and_res,
        Json::default(),
        "",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_no_payload_req_body_res_body,
        test_handler_req_body_and_res_body,
        Json::default(),
        "",
        "400 Bad Request"
    );

    test_error_in_connection_loop!(
        test_invalid_method_unity_res,
        test_handler_unity_res,
        (),
        "INVALID / HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_method_unity_res_body,
        test_handler_unity_res_body,
        (),
        "INVALID / HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_method_req_res,
        test_handler_with_req_and_res,
        Json::default(),
        "INVALID / HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_method_req_res_body,
        test_handler_with_req_and_res_body,
        Json::default(),
        "INVALID / HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_method_req_body_res,
        test_handler_req_body_and_res,
        Json::default(),
        "INVALID / HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_method_req_body_res_body,
        test_handler_req_body_and_res_body,
        Json::default(),
        "INVALID / HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );

    test_error_in_connection_loop!(
        test_invalid_uri_unity_res,
        test_handler_unity_res,
        (),
        "GET https:// HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_uri_unity_res_body,
        test_handler_unity_res_body,
        (),
        "GET https:// HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_uri_req_res,
        test_handler_with_req_and_res,
        Json::default(),
        "GET https:// HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_uri_req_res_body,
        test_handler_with_req_and_res_body,
        Json::default(),
        "GET https:// HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_uri_req_body_res,
        test_handler_req_body_and_res,
        Json::default(),
        "GET https:// HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_uri_req_body_res_body,
        test_handler_req_body_and_res_body,
        Json::default(),
        "GET https:// HTTP/1.1\r\n\r\n",
        "400 Bad Request"
    );

    test_error_in_connection_loop!(
        test_unimplemented_http_version_unity_res,
        test_handler_unity_res,
        (),
        "GET / HTTP/2\r\n\r\n",
        "505 HTTP Version Not Supported"
    );
    test_error_in_connection_loop!(
        test_unimplemented_http_version_unity_res_body,
        test_handler_unity_res_body,
        (),
        "GET / HTTP/2\r\n\r\n",
        "505 HTTP Version Not Supported"
    );
    test_error_in_connection_loop!(
        test_unimplemented_http_version_req_res,
        test_handler_with_req_and_res,
        Json::default(),
        "GET / HTTP/2\r\n\r\n",
        "505 HTTP Version Not Supported"
    );
    test_error_in_connection_loop!(
        test_unimplemented_http_version_req_res_body,
        test_handler_with_req_and_res_body,
        Json::default(),
        "GET / HTTP/2\r\n\r\n",
        "505 HTTP Version Not Supported"
    );
    test_error_in_connection_loop!(
        test_unimplemented_http_version_req_body_res,
        test_handler_req_body_and_res,
        Json::default(),
        "GET / HTTP/2\r\n\r\n",
        "505 HTTP Version Not Supported"
    );
    test_error_in_connection_loop!(
        test_unimplemented_http_version_req_body_res_body,
        test_handler_req_body_and_res_body,
        Json::default(),
        "GET / HTTP/2\r\n\r\n",
        "505 HTTP Version Not Supported"
    );

    test_error_in_connection_loop!(
        test_not_found_unity_res,
        test_handler_unity_res,
        (),
        "GET / HTTP/1.1\r\n\r\n",
        "404 Not Found"
    );
    test_error_in_connection_loop!(
        test_not_found_unity_res_body,
        test_handler_unity_res_body,
        (),
        "GET / HTTP/1.1\r\n\r\n",
        "404 Not Found"
    );
    test_error_in_connection_loop!(
        test_not_found_req_res,
        test_handler_with_req_and_res,
        Json::default(),
        "GET / HTTP/1.1\r\n\r\n",
        "404 Not Found"
    );
    test_error_in_connection_loop!(
        test_not_found_req_res_body,
        test_handler_with_req_and_res_body,
        Json::default(),
        "GET / HTTP/1.1\r\n\r\n",
        "404 Not Found"
    );
    test_error_in_connection_loop!(
        test_not_found_req_body_res,
        test_handler_req_body_and_res,
        Json::default(),
        "GET / HTTP/1.1\r\n\r\n",
        "404 Not Found"
    );
    test_error_in_connection_loop!(
        test_not_found_req_body_res_body,
        test_handler_req_body_and_res_body,
        Json::default(),
        "GET / HTTP/1.1\r\n\r\n",
        "404 Not Found"
    );

    test_error_in_connection_loop!(
        test_invalid_header_format_unity_res,
        test_handler_unity_res,
        (),
        "GET /aaaaa HTTP/1.1\r\n:Wrong\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_header_format_unity_res_body,
        test_handler_unity_res_body,
        (),
        "GET /aaaaa HTTP/1.1\r\n:Wrong\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_header_format_req_res,
        test_handler_with_req_and_res,
        Json::default(),
        "GET /aaaaa HTTP/1.1\r\n:Wrong\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_header_format_req_res_body,
        test_handler_with_req_and_res_body,
        Json::default(),
        "GET /aaaaa HTTP/1.1\r\n:Wrong\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_header_format_req_body_res,
        test_handler_req_body_and_res,
        Json::default(),
        "GET /aaaaa HTTP/1.1\r\n:Wrong\r\n",
        "400 Bad Request"
    );
    test_error_in_connection_loop!(
        test_invalid_header_format_req_body_res_body,
        test_handler_req_body_and_res_body,
        Json::default(),
        "GET /aaaaa HTTP/1.1\r\n:Wrong\r\n",
        "400 Bad Request"
    );

    test_error_in_connection_loop!(
        test_invalid_body_serialization_req_res,
        test_handler_with_req_and_res,
        Json::default(),
        "GET /aaaaa HTTP/1.1\r\nAccess:0\r\nwrong_body:true",
        "HTTP/1.1 422 Unprocessable Entity"
    );
    test_error_in_connection_loop!(
        test_invalid_body_serialization_req_res_body,
        test_handler_with_req_and_res_body,
        Json::default(),
        "GET /aaaaa HTTP/1.1\r\nAccess:0\r\nwrong_body:true",
        "HTTP/1.1 422 Unprocessable Entity"
    );
    test_error_in_connection_loop!(
        test_invalid_body_serialization_req_body_res,
        test_handler_req_body_and_res,
        Json::default(),
        "GET /aaaaa HTTP/1.1\r\nAccess:0\r\nwrong_body:true",
        "HTTP/1.1 422 Unprocessable Entity"
    );
    test_error_in_connection_loop!(
        test_invalid_body_serialization_req_body_res_body,
        test_handler_req_body_and_res_body,
        Json::default(),
        "GET /aaaaa HTTP/1.1\r\nAccess:0\r\nwrong_body:true",
        "HTTP/1.1 422 Unprocessable Entity"
    );
}
