use std::{fmt::Display, sync::Arc};

use crate::{
    handle_selector,
    handler::{encapsulate_runner, RefHandler, Runner},
    request::{self, HttpHeaderName, HttpHeaderValue, Request, Uri},
};
use async_std::{
    io::{BufReader, WriteExt},
    net::TcpStream,
    task,
};
use async_std::{
    net::{TcpListener, ToSocketAddrs},
    stream::StreamExt,
};
use futures::{AsyncBufReadExt, AsyncRead, AsyncWrite};
use handle_selector::HandlerSelect;

use request::Method;

#[derive(Default)]
pub struct Server<'a> {
    get: HandlerSelect<'a>,
    put: HandlerSelect<'a>,
    delete: HandlerSelect<'a>,
    post: HandlerSelect<'a>,
    trace: HandlerSelect<'a>,
    options: HandlerSelect<'a>,
    connect: HandlerSelect<'a>,
    patch: HandlerSelect<'a>,
    head: HandlerSelect<'a>,
}

impl<'a: 'static> Server<'a> {
    pub fn new() -> Self {
        Self {
            get: HandlerSelect::new(),
            put: HandlerSelect::new(),
            delete: HandlerSelect::new(),
            post: HandlerSelect::new(),
            trace: HandlerSelect::new(),
            options: HandlerSelect::new(),
            connect: HandlerSelect::new(),
            patch: HandlerSelect::new(),
            head: HandlerSelect::new(),
        }
    }

    pub fn add_handler<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        method: Method,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        match method {
            Method::GET => self.get.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            ),
            Method::PUT => self.put.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            ),
            Method::DELETE => self.delete.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            ),
            Method::POST => self.post.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            ),
            Method::TRACE => self.trace.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            ),
            Method::OPTIONS => self.options.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            ),
            Method::CONNECT => self.connect.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            ),
            Method::PATCH => self.patch.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            ),
            Method::HEAD => self.head.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            ),
            _ => (),
        }
    }

    pub fn get<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        self.add_handler(Method::GET, path, handler, deserializer, serializer)
    }

    pub fn post<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        self.add_handler(Method::POST, path, handler, deserializer, serializer)
    }

    pub fn put<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        self.add_handler(Method::PUT, path, handler, deserializer, serializer)
    }

    pub fn delete<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        self.add_handler(Method::DELETE, path, handler, deserializer, serializer)
    }

    pub fn trace<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        self.add_handler(Method::TRACE, path, handler, deserializer, serializer)
    }

    pub fn options<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        self.add_handler(Method::OPTIONS, path, handler, deserializer, serializer)
    }

    pub fn connect<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        self.add_handler(Method::CONNECT, path, handler, deserializer, serializer)
    }

    pub fn patch<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        self.add_handler(Method::PATCH, path, handler, deserializer, serializer)
    }

    pub fn head<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        self.add_handler(Method::HEAD, path, handler, deserializer, serializer)
    }

    pub fn all<FnIn, FnOut, Deserializer, Serializer, R>(
        &mut self,
        path: &'static str,
        handler: R,
        deserializer: &Deserializer,
        serializer: &Serializer,
    ) where
        R: 'static + Runner<(FnIn, Deserializer), (FnOut, Serializer)>,
        FnIn: 'static,
        FnOut: 'static,
        Deserializer: 'static,
        Serializer: 'static,
    {
        if let (None, None, None, None) = (
            self.get.get(path),
            self.post.get(path),
            self.put.get(path),
            self.delete.get(path),
        ) {
            self.get.insert(
                path,
                Box::new(encapsulate_runner(
                    handler.clone(),
                    deserializer,
                    serializer,
                )),
            );
            self.post.insert(
                path,
                Box::new(encapsulate_runner(
                    handler.clone(),
                    deserializer,
                    serializer,
                )),
            );
            self.put.insert(
                path,
                Box::new(encapsulate_runner(
                    handler.clone(),
                    deserializer,
                    serializer,
                )),
            );
            self.delete.insert(
                path,
                Box::new(encapsulate_runner(
                    handler.clone(),
                    deserializer,
                    serializer,
                )),
            );
            self.trace.insert(
                path,
                Box::new(encapsulate_runner(
                    handler.clone(),
                    deserializer,
                    serializer,
                )),
            );
            self.options.insert(
                path,
                Box::new(encapsulate_runner(
                    handler.clone(),
                    deserializer,
                    serializer,
                )),
            );
            self.patch.insert(
                path,
                Box::new(encapsulate_runner(
                    handler.clone(),
                    deserializer,
                    serializer,
                )),
            );
            self.head.insert(
                path,
                Box::new(encapsulate_runner(
                    handler.clone(),
                    deserializer,
                    serializer,
                )),
            );
            self.connect.insert(
                path,
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            );
        }
    }

    fn find_handler(&self, method: &Method, path: &str) -> Option<RefHandler<'_>> {
        match *method {
            Method::GET => self.get.get(path),
            Method::PUT => self.put.get(path),
            Method::POST => self.post.get(path),
            Method::DELETE => self.delete.get(path),
            Method::TRACE => self.trace.get(path),
            Method::OPTIONS => self.options.get(path),
            Method::CONNECT => self.connect.get(path),
            Method::PATCH => self.patch.get(path),
            Method::HEAD => self.head.get(path),
            _ => None,
        }
    }

    pub fn listen<A: ToSocketAddrs + Display>(self, addr: A) -> ListenResult<()> {
        task::block_on(accept_loop(self, addr))
    }
}
type ListenResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn accept_loop(
    server: Server<'static>,
    addr: impl ToSocketAddrs + Display,
) -> ListenResult<()> {
    let server = Arc::new(server);
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Start listening on {}", listener.local_addr().unwrap());
    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);
        handle_stream(server.clone(), stream);
    }
    Ok(())
}

fn handle_stream(
    server: Arc<Server<'static>>,
    mut stream: TcpStream,
) -> async_std::task::JoinHandle<()> {
    task::spawn(async move {
        let fut = connection_loop(server, &stream);
        if let Err(e) = fut.await {
            let formatted_error = format!("HTTP/1.1 {}", e);
            stream.write(formatted_error.as_bytes());
            eprintln!("{}", e);
        }
    })
}

const BAD_REQUEST: &str = "400 Bad Request";
const NOT_FOUND: &str = "404 Not Found";
const HTTP_VERSION_NOT_SUPPORTED: &str = "505 HTTP Version Not Supported";

async fn connection_loop(
    server: Arc<Server<'static>>,
    mut stream: impl AsyncRead + AsyncWrite + Unpin,
) -> ListenResult<()> {
    let buf_reader = BufReader::new(&mut stream);
    let mut lines = buf_reader.lines();
    let first_line = lines.next().await;

    let request_builder = Request::builder();
    let fl = match first_line {
        Some(first_line) => first_line.map_err(|_| BAD_REQUEST)?,
        None => Err(BAD_REQUEST)?,
    };

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

    let handler = server.find_handler(&method, &uri.to_string());
    let handler = match handler {
        Some(handler) => handler,
        None => Err(NOT_FOUND)?,
    };

    let mut request_builder = request_builder.method(method).uri(uri);

    while let Some(line) = lines.next().await {
        let line = line?;
        if line.is_empty() {
            break;
        }

        let splitted_header = line.split_once(':');
        match splitted_header {
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

    let mut body_string = String::with_capacity(100);
    while let Some(line) = lines.next().await {
        let line = line?;
        body_string.push_str(line.as_str());
    }

    let request = request_builder.body(body_string);

    let response = handler(request).await;

    let response_string = format!(
        "HTTP/1.1 {} {}\r\n{}\r\n{}",
        response.status().as_u16(),
        response.status().canonical_reason().unwrap(),
        response
            .headers()
            .into_iter()
            .fold(String::new(), |mut acc, (name, value)| {
                acc.push_str(format!("{}:{}\r\n", name, value.to_str().unwrap()).as_str());
                acc
            }),
        response.body()
    );

    stream.write_all(response_string.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

mod test_utils {
    use std::cmp::min;
    use std::pin::Pin;

    use futures::io::Error;
    use futures::task::{Context, Poll};
    use futures::{AsyncRead, AsyncWrite};

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
            self.read_data = self.read_data.drain(size..).collect::<Vec<_>>();
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
mod test_server {

    use async_std_test::async_test;
    use serde::{Deserialize, Serialize};

    use crate::{
        handler::{GenericResponse, Json},
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

    async fn run_test(server: &Server<'static>, req: Request<String>) -> GenericResponse {
        let method = req.method();

        let path = req.uri().path().to_string();

        let handler = server.find_handler(method, path.as_str());

        assert!(handler.is_some());

        handler.unwrap()(req).await
    }

    #[async_test]
    async fn test_handler_receiving_req_and_res() -> std::io::Result<()> {
        let mut server = Server::new();

        server.add_handler(
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
        let mut server = Server::new();

        server.all(
            "/test/all",
            test_handler_with_req_and_res,
            &Json::new(),
            &String::new(),
        );

        let req_body = serde_json::json!({ "correct": false }).to_string();
        let expected_res_body = serde_json::json!({ "correct": true }).to_string();

        let request = Request::builder().uri("/test/all").body(req_body.clone());

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
        let mut server = Server::new();

        server.$mtd(
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
    use serde::{Deserialize, Serialize};

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

    async fn run_test(server: Arc<Server<'static>>, test_config: TestConfig) {
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

        connection_loop(server, &mut stream).await.unwrap();

        // TODO: Implement ToString for Response
        let expected_contents = response.body();
        let expected_status_code = response.status().as_u16();
        let expected_status_message = response.status().canonical_reason();
        let expected_response = format!(
            "HTTP/1.1 {} {}\r\n\r\n{}",
            expected_status_code,
            expected_status_message.unwrap(),
            expected_contents
        );

        assert!(stream.write_data.starts_with(expected_response.as_bytes()));
    }

    macro_rules! test_connection_loop {
        ($test_name: tt, $fn: ident, $des: expr) => {
    #[async_test]
    async fn $test_name() -> std::io::Result<()> {
        let mut server = Server::new();
        server.all(
            "/aaaaa",
            $fn,
            &$des,
            &Json::default(),
        );

        let request = Request::builder()
            .uri("/aaaaa")
            .method(Method::GET)
            .body(serde_json::json!({"correct": false}).to_string());
        let response = Response::builder()
            .status(200)
            .body(serde_json::json!({"correct": true}).to_string());
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

    async fn test_for_error(server: Arc<Server<'static>>, request: String, response: String) {
        // TODO: Implement ToString for Request
        let mut stream = MockTcpStream {
            read_data: request.into_bytes(),
            write_data: vec![],
        };

        let error = connection_loop(server, &mut stream).await;

        // TODO: Implement ToString for Response
        println!("{:?}", error);
        match error {
            Ok(_) => assert!(stream.write_data.starts_with(response.as_bytes())),
            Err(err) => assert!(err.to_string().starts_with(response.as_str())),
        }
    }

    macro_rules! test_error_in_connection_loop {
        ($test_name:ident, $fn: ident, $des: expr, $send:literal, $expected: literal) => {
            #[async_test]
            async fn $test_name() -> std::io::Result<()> {
                let mut server = Server::new();
                server.all("/aaaaa", $fn, &$des, &Json::default());

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
