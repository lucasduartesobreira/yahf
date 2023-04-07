mod error;
mod handle_selector;
mod handler;

use crate::handler::{BodyExtractors, CRunner, RealRunner, RefAsyncHandler};
use handle_selector::HandlerSelect;

#[derive(Default)]
pub struct Server<'a> {
    get: HandlerSelect<'a>,
    put: HandlerSelect<'a>,
    delete: HandlerSelect<'a>,
    post: HandlerSelect<'a>,
}

pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

impl<'a> Server<'a> {
    pub fn new() -> Self {
        Self {
            get: HandlerSelect::new(),
            put: HandlerSelect::new(),
            delete: HandlerSelect::new(),
            post: HandlerSelect::new(),
        }
    }

    pub fn add_handler<Req, Res, BodyType, Extractor, R>(
        &mut self,
        method: Method,
        path: &'static str,
        handler: &'static R,
        extractor: &Extractor,
    ) where
        R: 'static + RealRunner<Req, Res, Extractor, BodyType>,
        Extractor: BodyExtractors<Item = BodyType>,
    {
        match method {
            Method::Get => self.get.insert(path, handler.create_runner(extractor)),
            Method::Put => self.put.insert(path, handler.create_runner(extractor)),
            Method::Delete => self.delete.insert(path, handler.create_runner(extractor)),
            Method::Post => self.post.insert(path, handler.create_runner(extractor)),
        }
    }

    pub fn get<Req, Res, BodyType, Extractor, R>(
        &mut self,
        path: &'static str,
        handler: &'static R,
        extractor: &Extractor,
    ) where
        R: 'static + RealRunner<Req, Res, Extractor, BodyType>,
        Extractor: BodyExtractors<Item = BodyType>,
    {
        self.add_handler(Method::Get, path, handler, extractor)
    }

    pub fn post<Req, Res, BodyType, Extractor, R>(
        &mut self,
        path: &'static str,
        handler: &'static R,
        extractor: &Extractor,
    ) where
        R: 'static + RealRunner<Req, Res, Extractor, BodyType>,
        Extractor: BodyExtractors<Item = BodyType>,
    {
        self.add_handler(Method::Post, path, handler, extractor)
    }

    pub fn put<Req, Res, BodyType, Extractor, R>(
        &mut self,
        path: &'static str,
        handler: &'static R,
        extractor: &Extractor,
    ) where
        R: 'static + RealRunner<Req, Res, Extractor, BodyType>,
        Extractor: BodyExtractors<Item = BodyType>,
    {
        self.add_handler(Method::Put, path, handler, extractor)
    }

    pub fn delete<Req, Res, BodyType, Extractor, R>(
        &mut self,
        path: &'static str,
        handler: &'static R,
        extractor: &Extractor,
    ) where
        R: 'static + RealRunner<Req, Res, Extractor, BodyType>,
        Extractor: BodyExtractors<Item = BodyType>,
    {
        self.add_handler(Method::Delete, path, handler, extractor)
    }

    pub fn all<Req, Res, BodyType, Extractor, R>(
        &mut self,
        path: &'static str,
        handler: &'static R,
        extractor: &Extractor,
    ) where
        R: 'static + RealRunner<Req, Res, Extractor, BodyType>,
        Extractor: BodyExtractors<Item = BodyType>,
    {
        if let (None, None, None, None) = (
            self.get.get(path),
            self.post.get(path),
            self.put.get(path),
            self.delete.get(path),
        ) {
            self.get.insert(path, handler.create_runner(extractor));
            self.post.insert(path, handler.create_runner(extractor));
            self.put.insert(path, handler.create_runner(extractor));
            self.delete.insert(path, handler.create_runner(extractor));
        }
    }

    pub fn find_handler(&'a self, method: Method, path: &'a str) -> Option<RefAsyncHandler<'a>> {
        match method {
            Method::Get => self.get.get(path),
            Method::Put => self.put.get(path),
            Method::Post => self.post.get(path),
            Method::Delete => self.delete.get(path),
        }
    }
}

#[cfg(test)]
mod tests {

    use async_std_test::async_test;
    use http::{Request, Response};
    use serde::{Deserialize, Serialize};

    use crate::{
        error::Error,
        handler::{GenericHttpResponse, Json, RequestWrapper},
        Method, Server,
    };

    #[derive(Clone, Debug, Deserialize, Serialize, Default)]
    struct TestStruct {
        correct: bool,
    }

    async fn test_handler_with_req_and_res(_req: Request<TestStruct>) -> GenericHttpResponse {
        Ok(Response::new(
            serde_json::to_string(&TestStruct { correct: true }).unwrap(),
        ))
    }

    async fn run_test(server: &Server<'_>, _req: Request<String>) -> GenericHttpResponse {
        let method = _req.method();
        let our_method = match *method {
            http::Method::GET => Method::Get,
            http::Method::POST => Method::Post,
            http::Method::PUT => Method::Put,
            http::Method::DELETE => Method::Delete,
            _ => unreachable!("Wrong tests for the moment"),
        };

        let path = _req.uri().path().to_string();

        println!("{}", path);

        let handler = server.find_handler(our_method, &path);

        assert!(handler.is_some());

        handler.unwrap()(RequestWrapper::new(_req.into_body())).await
    }

    #[async_test]
    async fn test_handler_receiving_req_and_res() -> std::io::Result<()> {
        let mut server = Server::new();

        server.add_handler(
            Method::Get,
            "/aaaa/bbbb",
            &test_handler_with_req_and_res,
            &Json::new(),
        );

        let request = Request::builder()
            .uri("/aaaa/bbbb")
            .body(serde_json::json!({ "correct": false }).to_string())
            .unwrap();

        let response = run_test(&server, request).await;

        assert!(
            response.is_ok(),
            "Mensagem de erro: {}",
            match response.err().unwrap() {
                Error::ParseBody(message) => message,
                Error::RequestError(error) => error._body,
            }
        );

        assert_eq!(
            *response.unwrap().body(),
            serde_json::json!({ "correct": true }).to_string()
        );

        Ok(())
    }

    #[async_test]
    async fn test_all_servers() -> std::io::Result<()> {
        let mut server = Server::new();

        server.all("/test/all", &test_handler_with_req_and_res, &Json::new());

        let req_body = serde_json::json!({ "correct": false }).to_string();
        let expected_res_body = serde_json::json!({ "correct": true }).to_string();

        let request = Request::builder()
            .uri("/test/all")
            .method(http::Method::GET)
            .body(req_body.clone())
            .unwrap();
        let response = run_test(&server, request).await;

        assert_eq!(*response.unwrap().body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(http::Method::POST)
            .body(req_body.clone())
            .unwrap();
        let response = run_test(&server, request).await;

        assert_eq!(*response.unwrap().body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(http::Method::PUT)
            .body(req_body.clone())
            .unwrap();
        let response = run_test(&server, request).await;

        assert_eq!(*response.unwrap().body(), expected_res_body.clone());

        let request = Request::builder()
            .uri("/test/all")
            .method(http::Method::DELETE)
            .body(req_body.clone())
            .unwrap();
        let response = run_test(&server, request).await;

        assert_eq!(*response.unwrap().body(), expected_res_body.clone());

        Ok(())
    }
}
