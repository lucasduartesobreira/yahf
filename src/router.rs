use std::sync::Arc;

use futures::Future;

use crate::{
    handle_selector::HandlerSelect,
    handler::{encapsulate_runner, RefHandler, Runner},
    middleware::{AfterMiddleware, MiddlewareFactory, PreMiddleware},
    request::{Method, Request},
    response::Response,
    result::InternalResult,
};

pub struct Router<PreM, AfterM> {
    middleware_factory: Arc<MiddlewareFactory<PreM, AfterM>>,
    get: HandlerSelect<'static>,
    put: HandlerSelect<'static>,
    delete: HandlerSelect<'static>,
    post: HandlerSelect<'static>,
    trace: HandlerSelect<'static>,
    options: HandlerSelect<'static>,
    connect: HandlerSelect<'static>,
    patch: HandlerSelect<'static>,
    head: HandlerSelect<'static>,
}

impl Router<(), ()> {
    pub fn new() -> Router<
        impl PreMiddleware<
            FutCallResponse = impl Future<Output = impl Into<InternalResult<Request<String>>>>,
        >,
        impl AfterMiddleware<
            FutCallResponse = impl Future<Output = impl Into<InternalResult<Response<String>>>>,
        >,
    > {
        Router {
            middleware_factory: Arc::new(MiddlewareFactory::new()),
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
}

macro_rules! method_insert {
    ($fn: ident, $method: expr) => {
        pub fn $fn<FnIn, FnOut, Deserializer, Serializer, R>(
            self,
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
            let built_with_middleware = self
                .middleware_factory
                .clone()
                .build(handler, deserializer, serializer);

            self.method(
                $method,
                path,
                built_with_middleware,
                &String::with_capacity(0),
                &String::with_capacity(0),
            )
        }
    };
}

impl<PreM, AfterM, FutP, FutA, ResultP, ResultA> Router<PreM, AfterM>
where
    PreM: PreMiddleware<FutCallResponse = FutP> + 'static,
    FutP: Future<Output = ResultP> + std::marker::Send + 'static,
    ResultP: Into<InternalResult<Request<String>>> + std::marker::Send + 'static,
    AfterM: AfterMiddleware<FutCallResponse = FutA> + 'static,
    FutA: Future<Output = ResultA> + std::marker::Send + 'static,
    ResultA: Into<InternalResult<Response<String>>> + std::marker::Send + 'static,
{
    pub fn router<OtherPreM, OtherAfterM, OtherFutA, OtherFutP, OtherResultP, OtherResultA>(
        mut self,
        router: Router<OtherPreM, OtherAfterM>,
    ) -> Router<PreM, AfterM>
    where
        OtherPreM: PreMiddleware<FutCallResponse = OtherFutP> + 'static,
        OtherAfterM: AfterMiddleware<FutCallResponse = OtherFutA> + 'static,
        OtherFutP: Future<Output = OtherResultP> + Send,
        OtherFutA: Future<Output = OtherResultA> + Send,
        OtherResultP: Into<InternalResult<Request<String>>> + Send,
        OtherResultA: Into<InternalResult<Response<String>>> + Send,
    {
        let [get, put, delete, post, trace, options, connect, patch, head] = [
            router.get,
            router.put,
            router.delete,
            router.post,
            router.trace,
            router.options,
            router.connect,
            router.patch,
            router.head,
        ]
        .map(|handler| {
            handler.apply(
                self.middleware_factory
                    .clone(),
            )
        });

        self.get.extend(get);
        self.put.extend(put);
        self.delete.extend(delete);
        self.post.extend(post);
        self.trace.extend(trace);
        self.options.extend(options);
        self.connect.extend(connect);
        self.patch.extend(patch);
        self.head.extend(head);

        self
    }

    pub fn pre<NewPreM, NewFut, NewResultP>(
        self,
        middleware: NewPreM,
    ) -> Router<impl PreMiddleware<FutCallResponse = impl Future<Output = NewResultP>>, AfterM>
    where
        NewPreM: PreMiddleware<FutCallResponse = NewFut>,
        NewFut: Future<Output = NewResultP>,
        NewResultP: Into<InternalResult<Request<String>>>,
    {
        let new_factory = self
            .middleware_factory
            .pre(middleware);
        Router {
            middleware_factory: Arc::new(new_factory),
            get: self.get,
            put: self.put,
            delete: self.delete,
            post: self.post,
            trace: self.trace,
            options: self.options,
            connect: self.connect,
            patch: self.patch,
            head: self.head,
        }
    }

    pub fn after<NewAfterM, NewFut, NewResultA>(
        self,
        middleware: NewAfterM,
    ) -> Router<PreM, impl AfterMiddleware<FutCallResponse = impl Future<Output = NewResultA>>>
    where
        NewAfterM: AfterMiddleware<FutCallResponse = NewFut>,
        NewFut: Future<Output = NewResultA>,
        NewResultA: Into<InternalResult<Response<String>>>,
    {
        let new_factory = self
            .middleware_factory
            .after(middleware);
        Router {
            middleware_factory: Arc::new(new_factory),
            get: self.get,
            put: self.put,
            delete: self.delete,
            post: self.post,
            trace: self.trace,
            options: self.options,
            connect: self.connect,
            patch: self.patch,
            head: self.head,
        }
    }

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
            _ => unreachable!("HTTP methods allowed: GET, POST, PUT, DELETE, TRACE, OPTIONS, CONNECT, PATCH, HEAD"),
        };

        self
    }

    method_insert!(get, Method::GET);
    method_insert!(put, Method::PUT);
    method_insert!(delete, Method::DELETE);
    method_insert!(post, Method::POST);
    method_insert!(trace, Method::TRACE);
    method_insert!(options, Method::OPTIONS);
    method_insert!(connect, Method::CONNECT);
    method_insert!(patch, Method::PATCH);
    method_insert!(head, Method::HEAD);

    pub fn all<FnIn, FnOut, Deserializer, Serializer, R>(
        self,
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
        let router = self.get(path, handler.clone(), deserializer, serializer);
        let router = router.put(path, handler.clone(), deserializer, serializer);
        let router = router.delete(path, handler.clone(), deserializer, serializer);
        let router = router.post(path, handler.clone(), deserializer, serializer);
        let router = router.trace(path, handler.clone(), deserializer, serializer);
        let router = router.options(path, handler.clone(), deserializer, serializer);
        let router = router.connect(path, handler.clone(), deserializer, serializer);
        let router = router.patch(path, handler.clone(), deserializer, serializer);

        router.head(path, handler, deserializer, serializer)
    }

    #[allow(dead_code)]
    pub(crate) fn find_route(&self, method: &Method, path: &str) -> Option<RefHandler<'_>> {
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
}

#[cfg(test)]
mod test {
    mod runners {
        use crate::request::Request;
        use crate::response::Response;
        use crate::result::Result;

        pub async fn runner_void_string() -> String {
            "1".into()
        }

        pub async fn runner_void_resp_string() -> Response<String> {
            Response::new("2".into())
        }

        pub async fn runner_void_result_string() -> Result<String> {
            Ok("3".into()).into()
        }

        pub async fn runner_void_result_response_string() -> Result<Response<String>> {
            Ok(Response::new("4".into())).into()
        }

        pub async fn runner_string_string(_req: String) -> String {
            "5".into()
        }

        pub async fn runner_string_result_string(_req: String) -> Result<String> {
            Ok("6".into()).into()
        }

        pub async fn runner_string_res_string(_req: String) -> Response<String> {
            Response::new("7".into())
        }

        pub async fn runner_string_result_res_string(_req: String) -> Result<Response<String>> {
            Ok(Response::new("8".into())).into()
        }

        pub async fn runner_req_string_resp_string(_req: Request<String>) -> Response<String> {
            Response::new("9".into())
        }

        pub async fn runner_req_string_string(_req: Request<String>) -> String {
            "10".into()
        }

        pub async fn runner_encapsulate_string(req: String) -> String {
            format!("[{}]", req)
        }
    }

    mod utils {
        use crate::{
            handler::RefHandler,
            request::{Method, Request},
            response::Response,
            result::InternalResult,
        };

        pub fn create_request(body: String, method: Method) -> Request<String> {
            Request::builder()
                .method(method)
                .header("Content-Length", body.len())
                .body(body)
        }

        pub fn test_runner_response(body: String, expected_body: String) {
            assert!(body == expected_body);
        }

        pub async fn run_runner(
            runner: RefHandler<'_>,
            request: InternalResult<Request<String>>,
        ) -> InternalResult<Response<String>> {
            runner.call(request).await
        }
    }

    macro_rules! build_router {
        ($id: ident, [$pre: expr]) => {
            let $id = Router::pre($id, $pre);
        };
        ($id: ident, ($after: expr)) => {
            let $id = Router::after($id, $after);
        };
        ($id: ident, [$($pre: expr),+]) => {
            $(build_router!($id, [$pre]);)+
        };
        ($id: ident, ($($after: expr),+)) => {
            $(build_router!($id, ($after));)+
        };
        ($id: ident, [$($pre: expr),*], ($($after:expr),*)) => {
            $(build_router!($id,[$pre]);)*
            $(build_router!($id,($after));)*
        }
    }

    macro_rules! test_router_insert_and_find {
        ($test_name: ident, $router_method: expr, $method: expr, $runner: expr,$des: expr, $body: literal, $expected_body: literal, [$($pre: expr),*], ($($after:expr),*)) => {
            #[tokio::test]
            async fn $test_name() -> std::io::Result<()> {
                let request = super::utils::create_request($body.to_owned(), $method);
                let router = Router::new();

                build_router!(
                    router,
                    [$($pre),*],
                    ($($after),*)
                );
                let router = $router_method(router, "/path/to", $runner, $des, &String::with_capacity(0));
                let handler = Router::find_route(&router, request.method(), "/path/to");

                assert!(handler.is_some());

                let response = super::utils::run_runner(handler.unwrap(), request.into()).await;

                super::utils::test_runner_response(
                    response.map_or_else(|err| err.into(), |res| res).body().to_owned(),
                    $expected_body.to_owned(),
                );

                Ok(())
            }
        };
        ($test_name: ident, $router_method: expr, $method: expr, $runner: expr,$des: expr, $body: literal, $expected_body: literal) => {
            test_router_insert_and_find!($test_name, $router_method, $method, $runner, $des, $body, $expected_body, [], ());
        };
    }

    macro_rules! test_router_insert_and_find_for_method {
        ($mod_name: ident, $router_method: expr, $method: expr) => {
            mod $mod_name {
                use crate::request::Method;
                use crate::router::Router;

                test_router_insert_and_find!(
                    test_insert_and_find_runner_void_string,
                    $router_method,
                    $method,
                    super::runners::runner_void_string,
                    &(),
                    "1",
                    "1"
                );
                test_router_insert_and_find!(
                    test_insert_and_find_runner_void_resp_string,
                    $router_method,
                    $method,
                    super::runners::runner_void_resp_string,
                    &(),
                    "2",
                    "2"
                );

                test_router_insert_and_find!(
                    test_insert_and_find_runner_void_result_string,
                    $router_method,
                    $method,
                    super::runners::runner_void_result_string,
                    &(),
                    "3",
                    "3"
                );

                test_router_insert_and_find!(
                    test_insert_and_find_runner_void_result_response_string,
                    $router_method,
                    $method,
                    super::runners::runner_void_result_response_string,
                    &(),
                    "4",
                    "4"
                );

                test_router_insert_and_find!(
                    test_insert_and_find_runner_string_string,
                    $router_method,
                    $method,
                    super::runners::runner_string_string,
                    &String::with_capacity(0),
                    "5",
                    "5"
                );

                test_router_insert_and_find!(
                    test_insert_and_find_runner_string_result_string,
                    $router_method,
                    $method,
                    super::runners::runner_string_result_string,
                    &String::with_capacity(0),
                    "6",
                    "6"
                );

                test_router_insert_and_find!(
                    test_insert_and_find_runner_string_res_string,
                    $router_method,
                    $method,
                    super::runners::runner_string_res_string,
                    &String::with_capacity(0),
                    "7",
                    "7"
                );

                test_router_insert_and_find!(
                    test_insert_and_find_runner_string_result_res_string,
                    $router_method,
                    $method,
                    super::runners::runner_string_result_res_string,
                    &String::with_capacity(0),
                    "8",
                    "8"
                );

                test_router_insert_and_find!(
                    test_insert_and_find_runner_req_string_resp_string,
                    $router_method,
                    $method,
                    super::runners::runner_req_string_resp_string,
                    &String::with_capacity(0),
                    "9",
                    "9"
                );

                test_router_insert_and_find!(
                    test_insert_and_find_runner_req_string_string,
                    $router_method,
                    $method,
                    super::runners::runner_req_string_string,
                    &String::with_capacity(0),
                    "10",
                    "10"
                );
            }
        };
    }

    test_router_insert_and_find_for_method!(test_insert_and_find_for_get, Router::get, Method::GET);
    test_router_insert_and_find_for_method!(test_insert_and_find_for_put, Router::put, Method::PUT);
    test_router_insert_and_find_for_method!(
        test_insert_and_find_for_delete,
        Router::delete,
        Method::DELETE
    );
    test_router_insert_and_find_for_method!(
        test_insert_and_find_for_post,
        Router::post,
        Method::POST
    );
    test_router_insert_and_find_for_method!(
        test_insert_and_find_for_trace,
        Router::trace,
        Method::TRACE
    );
    test_router_insert_and_find_for_method!(
        test_insert_and_find_for_options,
        Router::options,
        Method::OPTIONS
    );
    test_router_insert_and_find_for_method!(
        test_insert_and_find_for_connect,
        Router::connect,
        Method::CONNECT
    );
    test_router_insert_and_find_for_method!(
        test_insert_and_find_for_patch,
        Router::patch,
        Method::PATCH
    );
    test_router_insert_and_find_for_method!(
        test_insert_and_find_for_head,
        Router::head,
        Method::HEAD
    );

    mod middlewares {
        use crate::error::Error;
        use crate::request::Request;
        use crate::response::Response;
        use crate::result::Result;

        pub async fn pre_transform(req: Result<Request<String>>) -> Result<Request<String>> {
            req.into_inner()
                .map(|_| Request::new("PM1".into()))
                .into()
        }

        pub async fn pre_generate_error(_req: Result<Request<String>>) -> Result<Request<String>> {
            Err(Error::new("PM2".into(), 500)).into()
        }

        pub async fn pre_handle_error(req: Result<Request<String>>) -> Result<Request<String>> {
            Ok(req
                .into_inner()
                .unwrap_or(Request::new("PM3".into())))
            .into()
        }

        pub async fn after_transform(res: Result<Response<String>>) -> Result<Response<String>> {
            res.into_inner()
                .map(|_| Response::new("AM1".into()))
                .into()
        }

        pub async fn after_generate_error(
            _res: Result<Response<String>>,
        ) -> Result<Response<String>> {
            Err(Error::new("AM2".into(), 500)).into()
        }

        pub async fn after_handle_error(res: Result<Response<String>>) -> Result<Response<String>> {
            Ok(res
                .into_inner()
                .unwrap_or(Response::new("AM3".into())))
            .into()
        }
    }

    macro_rules! test_router_using_middlewares {
        ($mod_name:ident, $router_method: expr, $method: expr) => {
            mod $mod_name {
                use crate::{
                    request::Method,
                    router::{
                        test::{middlewares, runners},
                        Router,
                    },
                };

                test_router_insert_and_find!(
                    test_pre_transform,
                    $router_method,
                    $method,
                    runners::runner_encapsulate_string,
                    &String::with_capacity(0),
                    "Body",
                    "[PM1]",
                    [middlewares::pre_transform],
                    ()
                );

                test_router_insert_and_find!(
                    test_pre_generate_error,
                    $router_method,
                    $method,
                    runners::runner_encapsulate_string,
                    &String::with_capacity(0),
                    "Body",
                    "PM2",
                    [middlewares::pre_transform, middlewares::pre_generate_error],
                    ()
                );

                test_router_insert_and_find!(
                    test_handle_pre_middleware_error,
                    $router_method,
                    $method,
                    runners::runner_encapsulate_string,
                    &String::with_capacity(0),
                    "Body",
                    "[PM3]",
                    [
                        middlewares::pre_transform,
                        middlewares::pre_generate_error,
                        middlewares::pre_handle_error
                    ],
                    ()
                );

                test_router_insert_and_find!(
                    test_after_transform,
                    $router_method,
                    $method,
                    runners::runner_encapsulate_string,
                    &String::with_capacity(0),
                    "Body",
                    "AM1",
                    [
                        middlewares::pre_transform,
                        middlewares::pre_generate_error,
                        middlewares::pre_handle_error
                    ],
                    (middlewares::after_transform)
                );

                test_router_insert_and_find!(
                    test_after_generate_error,
                    $router_method,
                    $method,
                    runners::runner_encapsulate_string,
                    &String::with_capacity(0),
                    "Body",
                    "AM2",
                    [
                        middlewares::pre_transform,
                        middlewares::pre_generate_error,
                        middlewares::pre_handle_error
                    ],
                    (
                        middlewares::after_transform,
                        middlewares::after_generate_error
                    )
                );

                test_router_insert_and_find!(
                    test_handle_after_error,
                    $router_method,
                    $method,
                    runners::runner_encapsulate_string,
                    &String::with_capacity(0),
                    "Body",
                    "AM3",
                    [
                        middlewares::pre_transform,
                        middlewares::pre_generate_error,
                        middlewares::pre_handle_error
                    ],
                    (
                        middlewares::after_transform,
                        middlewares::after_generate_error,
                        middlewares::after_handle_error
                    )
                );

                test_router_insert_and_find!(
                    test_handle_pre_error_with_after_middleware,
                    $router_method,
                    $method,
                    runners::runner_encapsulate_string,
                    &String::with_capacity(0),
                    "Body",
                    "AM3",
                    [middlewares::pre_generate_error],
                    (middlewares::after_handle_error)
                );
            }
        };
    }

    test_router_using_middlewares!(
        test_router_using_middlewares_for_get,
        Router::get,
        Method::GET
    );
    test_router_using_middlewares!(
        test_router_using_middlewares_for_put,
        Router::put,
        Method::PUT
    );
    test_router_using_middlewares!(
        test_router_using_middlewares_for_delete,
        Router::delete,
        Method::DELETE
    );
    test_router_using_middlewares!(
        test_router_using_middlewares_for_post,
        Router::post,
        Method::POST
    );
    test_router_using_middlewares!(
        test_router_using_middlewares_for_trace,
        Router::trace,
        Method::TRACE
    );
    test_router_using_middlewares!(
        test_router_using_middlewares_for_options,
        Router::options,
        Method::OPTIONS
    );
    test_router_using_middlewares!(
        test_router_using_middlewares_for_connect,
        Router::connect,
        Method::CONNECT
    );
    test_router_using_middlewares!(
        test_router_using_middlewares_for_patch,
        Router::patch,
        Method::PATCH
    );
    test_router_using_middlewares!(
        test_router_using_middlewares_for_head,
        Router::head,
        Method::HEAD
    );
}
