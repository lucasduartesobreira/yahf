use std::sync::Arc;

use futures::{Future, TryFutureExt};

use crate::{
    handler::{InternalResult, Result, Runner},
    request::Request,
    response::Response,
};

impl From<Request<String>> for InternalResult<Request<String>> {
    fn from(val: Request<String>) -> Self {
        Ok(val)
    }
}

impl From<Response<String>> for InternalResult<Response<String>> {
    fn from(val: Response<String>) -> Self {
        Ok(val)
    }
}

pub trait PreMiddleware: Send + Sync + Copy {
    type FutCallResponse;
    fn call(&self, error: InternalResult<Request<String>>) -> Self::FutCallResponse;
}

impl<MidFn, Fut, CF> PreMiddleware for MidFn
where
    MidFn: Fn(Result<Request<String>>) -> Fut + Send + Sync + Copy,
    Fut: Future<Output = CF>,
    CF: Into<InternalResult<Request<String>>>,
{
    type FutCallResponse = Fut;

    #[inline(always)]
    fn call(&self, error: InternalResult<Request<String>>) -> Self::FutCallResponse {
        self(error.into())
    }
}

pub trait AfterMiddleware: Send + Sync + Copy {
    type FutCallResponse;
    fn call(&self, error: InternalResult<Response<String>>) -> Self::FutCallResponse;
}

impl<MidFn, Fut, CF> AfterMiddleware for MidFn
where
    MidFn: Fn(Result<Response<String>>) -> Fut + Send + Sync + Copy,
    Fut: Future<Output = CF>,
    CF: Into<InternalResult<Response<String>>>,
{
    type FutCallResponse = Fut;

    #[inline(always)]
    fn call(&self, error: InternalResult<Response<String>>) -> Self::FutCallResponse {
        self(error.into())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MiddlewareFactory<FPre, FAfter> {
    pre: FPre,
    after: FAfter,
}

impl MiddlewareFactory<(), ()> {
    #[inline(always)]
    async fn unit_pre_middleware(request: Result<Request<String>>) -> Result<Request<String>> {
        request
    }

    #[inline(always)]
    async fn unit_after_middleware(response: Result<Response<String>>) -> Result<Response<String>> {
        response
    }

    pub fn new() -> MiddlewareFactory<
        impl PreMiddleware<
            FutCallResponse = impl Future<Output = impl Into<InternalResult<Request<String>>>>,
        >,
        impl AfterMiddleware<
            FutCallResponse = impl Future<Output = impl Into<InternalResult<Response<String>>>>,
        >,
    > {
        MiddlewareFactory {
            pre: Self::unit_pre_middleware,
            after: Self::unit_after_middleware,
        }
    }
}

impl<FPre, FAfter, F, FA, CF, CFA> MiddlewareFactory<FPre, FAfter>
where
    FPre: PreMiddleware<FutCallResponse = F>,
    FAfter: AfterMiddleware<FutCallResponse = FA>,
    F: Future<Output = CF> + Send,
    CF: Into<InternalResult<Request<String>>> + Send,
    CFA: Into<InternalResult<Response<String>>> + Send,
    FA: Future<Output = CFA> + Send,
{
    #[inline(always)]
    pub fn pre<NewF: Future<Output = NewCF>, NewCF: Into<InternalResult<Request<String>>>>(
        self,
        other_pre: impl PreMiddleware<FutCallResponse = NewF> + Sync + Copy,
    ) -> MiddlewareFactory<impl PreMiddleware<FutCallResponse = impl Future<Output = NewCF>>, FAfter>
    {
        let pre = move |req: Result<Request<String>>| {
            let cloned_pre_middleware = self.pre;
            async move {
                let resp = cloned_pre_middleware.call(req.into()).await;
                let resp_internal_result: InternalResult<Request<String>> = resp.into();
                other_pre.call(resp_internal_result).await
            }
        };

        MiddlewareFactory {
            pre,
            after: self.after,
        }
    }

    #[inline(always)]
    pub fn after<NewF: Future<Output = NewCFA>, NewCFA: Into<InternalResult<Response<String>>>>(
        self,
        other_after: impl AfterMiddleware<FutCallResponse = NewF> + Sync + Copy,
    ) -> MiddlewareFactory<FPre, impl AfterMiddleware<FutCallResponse = impl Future<Output = NewCFA>>>
    {
        let after = move |res: Result<Response<String>>| {
            let cloned_after_middleware = self.after;
            async move {
                let resp = cloned_after_middleware.call(res.into()).await;
                other_after.call(resp.into()).await
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
            let pre = self.pre;
            let after = self.after;
            let runner = _runner.clone();
            async move {
                let req_updated: InternalResult<Request<String>> = pre.call(Ok(req)).await.into();
                let runner_resp = match req_updated {
                    Ok(req) => runner.call_runner(req).await,
                    Err(err) => Err(err),
                };
                let runner_resp_updated: InternalResult<Response<String>> =
                    after.call(runner_resp).await.into();
                runner_resp_updated
            }
            .map_ok_or_else(|e| e.into(), |ok| ok)
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use async_std_test::async_test;

    use crate::{
        error::Error,
        handler::{Result, Runner},
        middleware::MiddlewareFactory,
        request::Request,
        response::Response,
    };

    async fn pre_middleware(_req: Result<Request<String>>) -> Result<Request<String>> {
        Ok(Request::new(format!(
            "{}\nFrom middleware",
            _req.into_inner().unwrap().body()
        )))
        .into()
    }

    async fn pre_middleware_with_error(_req: Result<Request<String>>) -> Result<Request<String>> {
        Err(Error::new(
            "From middleware short-circuiting".to_owned(),
            200,
        ))
        .into()
    }

    async fn pre_middleware_error_handler(_: Result<Request<String>>) -> Result<Request<String>> {
        Err(Error::new("Error handled".to_owned(), 400)).into()
    }

    async fn test_handler(_req: Request<String>) -> Response<String> {
        Response::new(format!("{}\nFrom the handler", _req.body()))
    }

    async fn after_middleware(res: Result<Response<String>>) -> Result<Response<String>> {
        Ok(Response::new(format!(
            "{}\nFrom the after middleware",
            res.into_inner().unwrap().body()
        )))
        .into()
    }

    async fn after_middleware_with_error(
        res: Result<Response<String>>,
    ) -> Result<Response<String>> {
        Err(Error::new(
            format!(
                "{}\nFrom middleware short-circuiting",
                res.into_inner().unwrap().body()
            ),
            200,
        ))
        .into()
    }

    async fn after_middleware_error_handler(
        res: Result<Response<String>>,
    ) -> Result<Response<String>> {
        res.into_inner()
            .map_err(|_| Error::new("Error handled on after error".to_owned(), 400))
            .into()
    }

    async fn runner_with_error() -> Result<String> {
        Err(Error::new(
            "From runner with short-circuiting".to_owned(),
            400,
        ))
        .into()
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

        assert!(resp.unwrap().body() == "From pure request\nFrom middleware\nFrom the handler\nFrom the after middleware");

        Ok(())
    }

    #[async_test]
    async fn test_pre_middleware_with_short_circuit() -> std::io::Result<()> {
        let middleware = MiddlewareFactory::new();

        let middleware = middleware.pre(&pre_middleware_with_error);
        let arc_middleware = Arc::new(middleware);

        let updated_handler = arc_middleware.build(
            test_handler,
            &String::with_capacity(0),
            &String::with_capacity(0),
        );

        let resp = updated_handler
            .call_runner(Request::new("From pure request".to_owned()))
            .await;

        assert!(resp.unwrap().body() == "From middleware short-circuiting");

        Ok(())
    }

    #[async_test]
    async fn test_after_middleware_with_short_circuit() -> std::io::Result<()> {
        let middleware = MiddlewareFactory::new();

        let middleware = middleware.pre(&pre_middleware);
        let middleware = middleware.after(&after_middleware_with_error);
        let arc_middleware = Arc::new(middleware);

        let updated_handler = arc_middleware.build(
            test_handler,
            &String::with_capacity(0),
            &String::with_capacity(0),
        );

        let resp = updated_handler
            .call_runner(Request::new("From pure request".to_owned()))
            .await;

        assert!(resp.unwrap().body() == "From pure request\nFrom middleware\nFrom the handler\nFrom middleware short-circuiting");

        Ok(())
    }

    #[async_test]
    async fn test_pre_middleware_with_short_circuit_handled() -> std::io::Result<()> {
        let middleware = MiddlewareFactory::new();

        let middleware = middleware.pre(&pre_middleware_with_error);
        let middleware = middleware.pre(&pre_middleware_error_handler);
        let arc_middleware = Arc::new(middleware);

        let updated_handler = arc_middleware.build(
            test_handler,
            &String::with_capacity(0),
            &String::with_capacity(0),
        );

        let resp = updated_handler
            .call_runner(Request::new("From pure request".to_owned()))
            .await;

        assert!(resp.unwrap().body() == "Error handled");

        Ok(())
    }

    #[async_test]
    async fn test_after_middleware_with_short_circuit_handled() -> std::io::Result<()> {
        let middleware = MiddlewareFactory::new();

        let middleware = middleware.pre(&pre_middleware);
        let middleware = middleware.after(&after_middleware_with_error);
        let middleware = middleware.after(&after_middleware_error_handler);
        let arc_middleware = Arc::new(middleware);

        let updated_handler = arc_middleware.build(
            test_handler,
            &String::with_capacity(0),
            &String::with_capacity(0),
        );

        let resp = updated_handler
            .call_runner(Request::new("From pure request".to_owned()))
            .await;

        assert!(resp.unwrap().body() == "Error handled on after error");

        Ok(())
    }

    #[async_test]
    async fn test_runner_with_error_handled_by_middleware() -> std::io::Result<()> {
        let middleware = MiddlewareFactory::new();

        let middleware = middleware.pre(&pre_middleware);
        let middleware = middleware.after(&after_middleware_error_handler);
        let arc_middleware = Arc::new(middleware);

        let updated_handler =
            arc_middleware.build(runner_with_error, &(), &String::with_capacity(0));

        let resp = updated_handler
            .call_runner(Request::new("From pure request".to_owned()))
            .await;

        assert!(resp.unwrap().body() == "Error handled on after error");

        Ok(())
    }
}
