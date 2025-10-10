use std::{future::Future, pin::Pin, task::{Context, Poll}};
use std::convert::Infallible;
use tower::Service;
use miko_core::fast_builder::ResponseBuilder;
use crate::handler::handler::{Req, Resp};
use crate::handler::router::Router;

pub struct RouterSvc<S> {
    pub router: Router<S>,
}
impl<S> Clone for RouterSvc<S> {
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
        }
    }
}

impl<S: Send + Sync + 'static> Service<Req> for RouterSvc<S> {
    type Response = Resp;
    type Error = Infallible;
        type Future = Pin<Box<dyn Future<Output = Result<Resp, Infallible>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, mut req: Req) -> Self::Future {
            let method  = req.method().clone();
            let path    = req.uri().path().to_string();
            let result = self.router.find_handler(&method, &path).map(|h| h.clone());
            match result {
                Some((mut handler, params)) => Box::pin(async move {
                    req.extensions_mut().insert(params);
                    let resp = handler.call(req).await;
                    resp
                }),
                None => Box::pin(async move {ResponseBuilder::not_found()})
            }
    }
}
