use crate::router::HttpSvc;
use http_body_util::BodyExt;
use hyper::Request;
use hyper::body::Incoming;
use miko_core::{Req, Resp};
use std::convert::Infallible;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

#[derive(Clone)]
pub struct IncomingToInternal {
    pub inner: HttpSvc<Req>,
}

impl Service<Request<Incoming>> for IncomingToInternal {
    type Response = Resp;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Resp, Infallible>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req_incoming: Request<Incoming>) -> Self::Future {
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let req: Req = req_incoming.map(|inc| inc.map_err(|_| unreachable!()).boxed());
            inner.call(req).await
        })
    }
}
