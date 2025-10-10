use std::sync::Arc;

use hyper::{HeaderMap};
use hyper::http::request::Parts;

use crate::handler::handler::{PartsTag, ReqTag};
use crate::handler::{handler::Req};
pub type FRFut<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'static>>;
pub type FRPFut<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub trait FromRequest<S = (), M = ReqTag>: Send + Sync + 'static {
  fn from_request(req: Req, state: Arc<S>) -> FRFut<Self>;
}
pub trait FromRequestParts<S = ()>: Send + Sync + 'static {
  fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<Self>;
}

impl FromRequest for Req {
    fn from_request(req: Req, _state: Arc<()>) -> FRFut<Self> {
      Box::pin(async move {
          req
      })
    }
}

impl<S, T> FromRequest<S, PartsTag> for T
where
    S: Send + Sync + 'static,
    T: FromRequestParts<S> + Send + 'static,
{
    fn from_request(req: Req, state: Arc<S>) -> FRFut<Self> {
        let (mut parts, _) = req.into_parts();
        Box::pin(async move {
            T::from_request_parts(&mut parts, state).await
        })
    }
}

impl FromRequestParts for HeaderMap {
    fn from_request_parts(req: &mut Parts, _state: Arc<()>) -> FRFut<Self> {
        let headers = req.headers.clone();
        Box::pin(async move {
            headers 
        })
    }
}