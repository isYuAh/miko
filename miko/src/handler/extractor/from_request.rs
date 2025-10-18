use anyhow::Error;
use hyper::HeaderMap;
use hyper::http::request::Parts;
use std::sync::Arc;

use crate::handler::handler::Req;
use crate::handler::handler::{PartsTag, ReqTag};
pub type FRFut<T> = std::pin::Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'static>>;
pub type FRPFut<'a, T> = std::pin::Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>;
pub trait FromRequest<S = (), M = ReqTag>: Send + Sync + 'static {
    fn from_request(req: Req, state: Arc<S>) -> FRFut<Self>
    where
        Self: Sized;
}
pub trait FromRequestParts<S = ()>: Send + Sync + 'static {
    fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<Self>
    where
        Self: Sized;
}

impl<S> FromRequest<S> for Req {
    fn from_request(req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move { Ok(req) })
    }
}

impl<S, T> FromRequest<S, PartsTag> for T
where
    S: Send + Sync + 'static,
    T: FromRequestParts<S> + Send + 'static,
{
    fn from_request(req: Req, state: Arc<S>) -> FRFut<Self> {
        let (mut parts, _) = req.into_parts();
        Box::pin(async move { T::from_request_parts(&mut parts, state).await })
    }
}

impl<S> FromRequestParts<S> for HeaderMap {
    fn from_request_parts(req: &mut Parts, _state: Arc<S>) -> FRFut<Self> {
        let headers = req.headers.clone();
        Box::pin(async move { Ok(headers) })
    }
}

impl<S, T> FromRequestParts<S> for Option<T>
where
    S: Send + Sync + 'static,
    T: FromRequestParts<S> + Send + 'static,
{
    fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<Self> {
        Box::pin(async move {
            match T::from_request_parts(req, state).await {
                Ok(v) => Ok(Some(v)),
                Err(_) => Ok(None),
            }
        })
    }
}

impl<S, T> FromRequest<S> for Option<T>
where
    S: Send + Sync + 'static,
    T: FromRequest<S> + Send + 'static,
{
    fn from_request(req: Req, state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            match T::from_request(req, state).await {
                Ok(v) => Ok(Some(v)),
                Err(_e) => Ok(None),
            }
        })
    }
}

impl<S, T> FromRequest<S> for Result<T, Error>
where
    S: Send + Sync + 'static,
    T: FromRequest<S> + Send + 'static,
{
    fn from_request(req: Req, state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            match T::from_request(req, state).await {
                Ok(v) => Ok(Ok(v)),
                Err(_e) => Ok(Err(_e)),
            }
        })
    }
}

impl<S, T> FromRequestParts<S> for Result<T, Error>
where
    S: Send + Sync + 'static,
    T: FromRequestParts<S> + Send + 'static,
{
    fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<Self> {
        Box::pin(async move {
            match T::from_request_parts(req, state).await {
                Ok(v) => Ok(Ok(v)),
                Err(_e) => Err(_e),
            }
        })
    }
}
