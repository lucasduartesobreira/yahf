use crate::{
    handle_selector,
    handler::{encapsulate_runner, RefHandler, Runner},
    request::{self, Request, Uri},
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

impl<'a> Server<'a> {
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

    pub fn listen<A: ToSocketAddrs>(&'static self, addr: A) -> ListenResult<()> {
        task::block_on(accept_loop(self, addr))
    }
}
type ListenResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn accept_loop(server: &'static Server<'_>, addr: impl ToSocketAddrs) -> ListenResult<()> {
    let listener = TcpListener::bind(addr).await.unwrap();
    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        handle_stream(server, stream);
    }
    Ok(())
}

fn handle_stream(
    server: &'static Server<'_>,
    _stream: TcpStream,
) -> async_std::task::JoinHandle<()> {
    task::spawn(async move {
        if let Err(e) = connection_loop(server, _stream).await {
            eprintln!("{}", e)
        }
    })
}

async fn connection_loop(
    server: &Server<'_>,
    mut stream: impl AsyncRead + AsyncWrite + Unpin,
) -> ListenResult<()> {
    let buf_reader = BufReader::new(&mut stream);
    let mut lines = buf_reader.lines();
    let first_line = lines.next().await;

    let request_builder = Request::builder();
    let fl = match first_line {
        Some(first_line) => first_line?,
        None => Err("Fuck this")?,
    };

    let mut splitted_fl = fl.split(' ');
    let method = match splitted_fl.next() {
        Some(mtd) => Method::try_from(mtd)?,
        None => Err("Wrong request structure")?,
    };
    let request_builder = request_builder.method(method);

    let uri = match splitted_fl.next() {
        Some(mtd) => Uri::try_from(mtd)?,
        None => Err("Wrong request structure")?,
    };
    let request_builder = request_builder.uri(uri);

    match splitted_fl.next() {
        Some("HTTP/1.1") => (),
        _ => Err("Wrong request structure")?,
    };

    let mut request_with_headers = request_builder;

    while let Some(line) = lines.next().await {
        let line = line?;
        if line.is_empty() {
            break;
        }

        let splitted_header = line.split_once(':');
        match splitted_header {
            Some((header, value)) => {
                request_with_headers = request_with_headers.header(header.trim(), value.trim());
            }
            None => Err("Wrong header format")?,
        }
    }

    let mut body_string = String::with_capacity(100);
    while let Some(line) = lines.next().await {
        let line = line?;
        body_string.push_str(line.as_str());
    }

    let request = request_with_headers.body(body_string);

    let handler = server.find_handler(request.method(), request.uri().path());
    let handler = match handler {
        Some(handler) => handler,
        None => Err("Not found")?,
    };

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

#[cfg(test)]
mod test_add_handlers {

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
}

#[cfg(test)]
mod test_listen {
    use std::cmp::min;
    use std::pin::Pin;

    use async_std_test::async_test;
    use futures::io::Error;
    use futures::task::{Context, Poll};
    use futures::{AsyncRead, AsyncWrite};
    use serde::{Deserialize, Serialize};

    use crate::response::Response;
    use crate::server::connection_loop;
    use crate::{handler::Json, request::Request, server::Server};

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct TestStruct {
        correct: bool,
    }

    async fn test_handler_with_req_and_res(_req: Request<TestStruct>) -> Response<TestStruct> {
        Response::new(TestStruct { correct: true })
    }

    struct MockTcpStream {
        read_data: Vec<u8>,
        write_data: Vec<u8>,
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

    #[async_test]
    async fn test_connection_loop() -> std::io::Result<()> {
        let mut server = Server::new();
        server.all(
            "/aaaaa",
            test_handler_with_req_and_res,
            &Json::default(),
            &Json::default(),
        );

        let input_bytes = "GET /aaaaa HTTP/1.1\r\n\r\n{\"correct\": false}";

        let mut stream = MockTcpStream {
            read_data: input_bytes.as_bytes().to_vec(),
            write_data: Vec::new(),
        };

        connection_loop(&server, &mut stream).await.unwrap();

        let expected_contents = "{\"correct\":true}";
        let expected_response = format!("HTTP/1.1 200 OK\r\n\r\n{}", expected_contents);

        assert!(stream.write_data.starts_with(expected_response.as_bytes()));
        Ok(())
    }
}
