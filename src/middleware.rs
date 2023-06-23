use std::sync::Arc;

use futures::Future;

use crate::{handler::Runner, request::Request, response::Response};

type ControlFlow<T> = Result<T, Response<String>>;

pub trait PreMiddleware: Send + Sync + Clone {
    type FutCallResponse;
    fn call(&self, req: Request<String>) -> Self::FutCallResponse;
}

impl From<Request<String>> for ControlFlow<Request<String>> {
    fn from(val: Request<String>) -> Self {
        Ok(val)
    }
}

impl From<Response<String>> for ControlFlow<Request<String>> {
    fn from(val: Response<String>) -> Self {
        Err(val)
    }
}

impl<MidFn, Fut, CF> PreMiddleware for MidFn
where
    MidFn: Fn(Request<String>) -> Fut + Send + Sync + Clone,
    Fut: Future<Output = CF>,
    CF: Into<ControlFlow<Request<String>>>,
{
    type FutCallResponse = Fut;
    #[inline(always)]
    fn call(&self, req: Request<String>) -> Self::FutCallResponse {
        self(req)
    }
}

pub trait AfterMiddleware: Send + Sync + Clone {
    type FutCallResponse;
    fn call(&self, req: Response<String>) -> Self::FutCallResponse;
}

impl<MidFn, Fut> AfterMiddleware for MidFn
where
    MidFn: Fn(Response<String>) -> Fut + Send + Sync + Clone,
    Fut: Future<Output = Response<String>>,
{
    type FutCallResponse = Fut;
    #[inline(always)]
    fn call(&self, req: Response<String>) -> Self::FutCallResponse {
        self(req)
    }
}

#[derive(Debug, Default)]
pub struct MiddlewareFactory<FPre, FAfter> {
    pre: FPre,
    after: FAfter,
}

impl MiddlewareFactory<(), ()> {
    #[inline(always)]
    async fn unit_pre_middleware(request: Request<String>) -> Request<String> {
        request
    }

    #[inline(always)]
    async fn unit_after_middleware(response: Response<String>) -> Response<String> {
        response
    }

    pub fn new() -> MiddlewareFactory<
        impl PreMiddleware<FutCallResponse = impl Future<Output = Request<String>>>,
        impl AfterMiddleware<FutCallResponse = impl Future<Output = Response<String>>>,
    > {
        MiddlewareFactory {
            pre: Self::unit_pre_middleware,
            after: Self::unit_after_middleware,
        }
    }

    pub fn pre<F: Future<Output = Request<String>>, UPM: PreMiddleware<FutCallResponse = F>>(
        self,
        pre: UPM,
    ) -> MiddlewareFactory<UPM, ()> {
        MiddlewareFactory { pre, after: () }
    }

    pub fn after<
        F: Future<Output = Response<String>>,
        UAFM: AfterMiddleware<FutCallResponse = F>,
    >(
        self,
        after: UAFM,
    ) -> MiddlewareFactory<(), UAFM> {
        MiddlewareFactory { pre: (), after }
    }
}

impl<FPre, FAfter, F, FA, CF> MiddlewareFactory<FPre, FAfter>
where
    FPre: PreMiddleware<FutCallResponse = F> + 'static,
    FAfter: AfterMiddleware<FutCallResponse = FA> + 'static,
    F: Future<Output = CF> + Send,
    CF: Into<ControlFlow<Request<String>>> + Send,
    FA: Future<Output = Response<String>> + Send,
{
    #[inline(always)]
    pub fn pre<NewF: Future<Output = NewCF>, NewCF: Into<ControlFlow<Request<String>>>>(
        self,
        other_pre: &'static (impl PreMiddleware<FutCallResponse = NewF> + Sync),
    ) -> MiddlewareFactory<
        impl PreMiddleware<FutCallResponse = impl Future<Output = ControlFlow<Request<String>>>>,
        FAfter,
    > {
        let pre = move |req| {
            let cloned_pre_middleware = self.pre.clone();
            async move {
                let resp = cloned_pre_middleware.call(req).await;

                match resp.into() {
                    Ok(req) => other_pre.call(req).await.into(),
                    Err(resp) => resp.into(),
                }
            }
        };

        MiddlewareFactory {
            pre,
            after: self.after,
        }
    }

    #[inline(always)]
    pub fn after<NewF: Future<Output = Response<String>>>(
        self,
        other_after: &'static (impl AfterMiddleware<FutCallResponse = NewF> + Sync),
    ) -> MiddlewareFactory<
        FPre,
        impl AfterMiddleware<FutCallResponse = impl Future<Output = Response<String>>>,
    > {
        let after = move |res| {
            let cloned_after_middleware = self.after.clone();
            async move {
                let resp = cloned_after_middleware.call(res).await;
                other_after.call(resp).await
            }
        };

        MiddlewareFactory {
            pre: self.pre,
            after,
        }
    }

    pub fn build<R, FnInput, FnOutput, Deserializer, Serializer>(
        self: Arc<Self>,
        _runner: R,
        _deserializer: &Deserializer,
        _serializer: &Serializer,
    ) -> impl Runner<(Request<String>, String), (Response<String>, String)>
    where
        R: Runner<(FnInput, Deserializer), (FnOutput, Serializer)> + 'static,
    {
        move |req| {
            let pre = self.pre.clone();
            let after = self.after.clone();
            let runner = _runner.clone();
            async move {
                let req_updated = pre.call(req).await;
                match req_updated.into() {
                    Ok(req) => {
                        let a = runner.call_runner(req).await;
                        let res_updated = after.call(a);
                        res_updated.await
                    }
                    Err(resp) => resp,
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use async_std_test::async_test;

    use crate::{
        handler::Runner, middleware::MiddlewareFactory, request::Request, response::Response,
    };

    use super::ControlFlow;

    async fn pre_middleware(_req: Request<String>) -> Request<String> {
        Request::new(format!("{}\nFrom middleware", _req.body()))
    }

    async fn pre_middleware_short_circuiting(
        _req: Request<String>,
    ) -> ControlFlow<Request<String>> {
        Err(Response::new("From middleware short-circuiting".to_owned()))
    }

    async fn test_handler(_req: Request<String>) -> Response<String> {
        Response::new(format!("{}\nFrom the handler", _req.body()))
    }

    async fn after_middleware(res: Response<String>) -> Response<String> {
        Response::new(format!("{}\nFrom the after middleware", res.body()))
    }

    #[async_test]
    async fn test_middleware_creation() -> std::io::Result<()> {
        let middleware = MiddlewareFactory::new();

        let middleware = middleware.pre(&pre_middleware);
        let middleware = middleware.after(&after_middleware);
        let arc_middleware = Arc::new(middleware);

        let updated_handler = arc_middleware.build(
            test_handler,
            &String::with_capacity(0),
            &String::with_capacity(0),
        );

        let resp = updated_handler
            .call_runner(Request::new("From pure request".to_owned()))
            .await;

        assert!(resp.body() == "From pure request\nFrom middleware\nFrom the handler\nFrom the after middleware");

        Ok(())
    }

    #[async_test]
    async fn test_pre_middleware_with_short_circuit() -> std::io::Result<()> {
        let middleware = MiddlewareFactory::new();

        let middleware = middleware.pre(&pre_middleware_short_circuiting);
        let arc_middleware = Arc::new(middleware);

        let updated_handler = arc_middleware.build(
            test_handler,
            &String::with_capacity(0),
            &String::with_capacity(0),
        );

        let resp = updated_handler
            .call_runner(Request::new("From pure request".to_owned()))
            .await;

        assert!(resp.body() == "From middleware short-circuiting");

        Ok(())
    }
}
