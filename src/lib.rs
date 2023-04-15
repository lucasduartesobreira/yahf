mod error;
mod handle_selector;
mod handler;

use crate::handler::{encapsulate_runner, Method, RefHandler, Runner};
use handle_selector::HandlerSelect;

#[derive(Default)]
pub struct Server<'a> {
    get: HandlerSelect<'a>,
    put: HandlerSelect<'a>,
    delete: HandlerSelect<'a>,
    post: HandlerSelect<'a>,
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
                Box::new(encapsulate_runner(handler, deserializer, serializer)),
            );
        }
    }

    #[allow(dead_code)]
    fn find_handler(&'a self, method: &Method, path: &'a str) -> Option<RefHandler<'a>> {
        match *method {
            Method::GET => self.get.get(path),
            Method::PUT => self.put.get(path),
            Method::POST => self.post.get(path),
            Method::DELETE => self.delete.get(path),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {

    use async_std_test::async_test;
    use serde::{Deserialize, Serialize};

    use crate::{
        handler::{GenericResponse, Json, Method, Request, Response},
        Server,
    };

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct TestStruct {
        correct: bool,
    }

    async fn test_handler_with_req_and_res(_req: Request<TestStruct>) -> GenericResponse {
        Response::new(serde_json::to_string(&TestStruct { correct: true }).unwrap())
    }

    async fn run_test(server: &Server<'_>, req: Request<String>) -> GenericResponse {
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
    async fn test_all_servers() -> std::io::Result<()> {
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

        Ok(())
    }
}
