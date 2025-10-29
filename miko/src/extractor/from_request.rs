use crate::error::AppError;
use hyper::HeaderMap;
use hyper::http::request::Parts;
use std::sync::Arc;

use crate::handler::Req;
use crate::handler::{PartsTag, ReqTag};

/// FromRequest 返回值的异步类型别名（拥有请求体）
pub type FRFut<T> = std::pin::Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'static>>;
/// FromRequestParts 返回值的异步类型别名（仅解析请求头与路径等）
pub type FRPFut<'a, T> = std::pin::Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'a>>;

/// 基于完整 Request 的提取器
///
/// 实现该 trait 可以自定义从请求中提取参数的逻辑。通常用于需要读取 Body 的场景。
pub trait FromRequest<S = (), M = ReqTag>: Send + Sync + 'static {
    fn from_request(req: Req, state: Arc<S>) -> FRFut<Self>
    where
        Self: Sized;
}

/// 基于 request::Parts 的提取器
///
/// 仅能访问方法、路径、头部、扩展等，不消耗 Body，适合 Header/Query/Path 等提取。
pub trait FromRequestParts<S = ()>: Send + Sync + 'static {
    fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<'_, Self>
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

impl<S> FromRequestParts<S> for Parts {
    fn from_request_parts(req: &mut Parts, _state: Arc<S>) -> FRFut<Self> {
        let req = req.clone();
        Box::pin(async move { Ok(req) })
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
    fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<'_, Self> {
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

impl<S, T> FromRequest<S> for Result<T, AppError>
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

impl<S, T> FromRequestParts<S> for Result<T, AppError>
where
    S: Send + Sync + 'static,
    T: FromRequestParts<S> + Send + 'static,
{
    fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<'_, Self> {
        Box::pin(async move {
            match T::from_request_parts(req, state).await {
                Ok(v) => Ok(Ok(v)),
                Err(_e) => Err(_e),
            }
        })
    }
}
