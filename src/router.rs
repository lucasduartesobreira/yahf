use futures::Future;

use crate::{
    handle_selector::HandlerSelect,
    handler::InternalResult,
    middleware::{AfterMiddleware, MiddlewareFactory, PreMiddleware},
    request::Request,
    response::Response,
};

pub struct Router<MPre, MAfter> {
    middleware_factory: MiddlewareFactory<MPre, MAfter>,
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
            middleware_factory: MiddlewareFactory::new(),
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

impl<MPre, MAfter, FutP, FutA, CFP, CFA> Router<MPre, MAfter>
where
    MPre: PreMiddleware<FutCallResponse = FutP>,
    FutP: Future<Output = CFP> + std::marker::Send,
    CFP: Into<InternalResult<Request<String>>> + std::marker::Send,
    MAfter: AfterMiddleware<FutCallResponse = FutA>,
    FutA: Future<Output = CFA> + std::marker::Send,
    CFA: Into<InternalResult<Response<String>>> + std::marker::Send,
{
    pub fn extend<OtherMPre, OtherMAfter, OtherFutA, OtherFutP, OtherCFP, OtherCFA>(
        mut self,
        router: Router<OtherMPre, OtherMAfter>,
    ) -> Router<
        impl PreMiddleware<
            FutCallResponse = impl Future<Output = impl Into<InternalResult<Request<String>>>>,
        >,
        impl AfterMiddleware<
            FutCallResponse = impl Future<Output = impl Into<InternalResult<Response<String>>>>,
        >,
    >
    where
        OtherMPre: PreMiddleware<FutCallResponse = OtherFutP> + 'static,
        OtherMAfter: AfterMiddleware<FutCallResponse = OtherFutA> + 'static,
        OtherFutP: Future<Output = OtherCFP> + Send,
        OtherFutA: Future<Output = OtherCFA> + Send,
        OtherCFP: Into<InternalResult<Request<String>>> + Send,
        OtherCFA: Into<InternalResult<Response<String>>> + Send,
    {
        let (other_pre, other_after) = router.middleware_factory.into_parts();
        let combined_middleware = self.middleware_factory.pre(other_pre).after(other_after);

        self.get.extend(router.get);
        self.put.extend(router.put);
        self.delete.extend(router.delete);
        self.post.extend(router.post);
        self.trace.extend(router.trace);
        self.options.extend(router.options);
        self.connect.extend(router.connect);
        self.patch.extend(router.patch);
        self.head.extend(router.head);

        Router {
            middleware_factory: combined_middleware,
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
}
