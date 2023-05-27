use std::sync::Arc;

use futures::Future;

use crate::{
    handler::{encapsulate_runner, BoxedHandler, Runner},
    request::Request,
};

pub trait PreMiddleware: Send + Sync + Clone {
    type FutCallResponse;
    fn call(&self, req: Request<String>) -> Self::FutCallResponse;
}

impl<MidFn, Fut> PreMiddleware for MidFn
where
    MidFn: Fn(Request<String>) -> Fut + Send + Sync + Clone,
    Fut: Future<Output = Request<String>>,
{
    type FutCallResponse = Fut;
    #[inline(always)]
    fn call(&self, req: Request<String>) -> Self::FutCallResponse {
        self(req)
    }
}

impl<FPre, FAfter, F> MiddlewareFactory<FPre, FAfter>
where
    FPre: PreMiddleware<FutCallResponse = F> + 'static,
    FAfter: 'static + Send + std::marker::Sync,
    F: Future<Output = Request<String>> + Send,
{
    #[inline(always)]
    pub fn pre<NewF: Future<Output = Request<String>>>(
        self,
        _before: &'static (impl PreMiddleware<FutCallResponse = NewF> + Sync),
    ) -> MiddlewareFactory<
        impl PreMiddleware<FutCallResponse = impl Future<Output = Request<String>>>,
        FAfter,
    > {
        let pre = move |req| {
            let cloned_pre_middleware = self.pre.clone();
            async move {
                let resp = cloned_pre_middleware.call(req).await;
                _before.call(resp).await
            }
        };

        MiddlewareFactory {
            pre,
            after: self.after,
        }
    }

    pub fn build<R, FnInput, FnOutput, Deserializer, Serializer>(
        self: Arc<Self>,
        _runner: R,
        _deserializer: &Deserializer,
        _serializer: &Serializer,
    ) -> BoxedHandler
    where
        R: Runner<(FnInput, Deserializer), (FnOutput, Serializer)> + 'static,
    {
        let handler = move |req| {
            let pre = self.pre.clone();
            let runner = _runner.clone();
            async move {
                let req_updated = pre.call(req).await;
                let a = runner.call_runner(req_updated);
                a.await
            }
        };

        Box::new(encapsulate_runner(
            handler,
            &String::with_capacity(0),
            &String::with_capacity(0),
        ))
    }
}

#[derive(Debug, Default)]
pub struct MiddlewareFactory<FPre, FAfter> {
    pre: FPre,
    after: FAfter,
}

impl MiddlewareFactory<(), ()> {
    pub fn new() -> Self {
        Self { pre: (), after: () }
    }

    pub fn pre<F: Future<Output = Request<String>>, UPM: PreMiddleware<FutCallResponse = F>>(
        self,
        pre: UPM,
    ) -> MiddlewareFactory<UPM, ()> {
        MiddlewareFactory { pre, after: () }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use async_std_test::async_test;

    use crate::{middleware::MiddlewareFactory, request::Request, response::Response};

    async fn pre_middleware(_req: Request<String>) -> Request<String> {
        Request::new(format!("{}\rFrom middleware", _req.body()))
    }

    async fn test_handler(_req: Request<String>) -> Response<String> {
        Response::new(format!("{}\rFrom the handler", _req.body()))
    }

    #[async_test]
    async fn test_pre_middleware() -> std::io::Result<()> {
        let middleware = MiddlewareFactory::new();

        let middleware = middleware.pre(pre_middleware);
        let arc_middleware = Arc::new(middleware);

        let updated_handler = arc_middleware.build(
            test_handler,
            &String::with_capacity(0),
            &String::with_capacity(0),
        );

        let resp = updated_handler(Request::new("From pure request".to_owned())).await;

        assert!(resp.body() == "From pure request\rFrom middleware\rFrom the handler");

        Ok(())
    }
}
