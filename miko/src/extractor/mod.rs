use crate::extractor::from_request::FRPFut;
use crate::extractor::path_params::PathParams;
use crate::handler::handler::Req;
use anyhow::anyhow;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::http::Extensions;
use hyper::http::request::Parts;
use hyper::{Method, Uri};
use miko_core::fast_builder::boxed_err;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::sync::Arc;
use crate::extractor::from_request::{FRFut, FromRequest, FromRequestParts};

/// JSON 请求体提取器，将请求体反序列化为 T
#[derive(Debug)]
pub struct Json<T>(pub T);
/// URL 查询字符串提取器，将 ?a=1&b=2 解析为 T
pub struct Query<T>(pub T);
/// 路径参数提取器，从 PathParams 中提取首个段并转换为 T
pub struct Path<T>(pub T);
/// 全局状态提取器，配合 Router::with_state 提供的 Arc<T>
pub struct State<T>(pub Arc<T>);
/// application/x-www-form-urlencoded 表单提取器
pub struct Form<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    T: DeserializeOwned + Send + Sync + 'static,
{
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        let _ = _state;
        Box::pin(async move {
            let body = req.body_mut().collect().await.unwrap().to_bytes();
            let json = serde_json::from_slice(&body);
            match json {
                Ok(json) => Ok(Json(json)),
                Err(err) => Err(err.into()),
            }
        })
    }
}

impl<S, T> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned + Send + Sync + 'static,
{
    fn from_request_parts(req: &mut Parts, _state: Arc<S>) -> FRFut<Self> {
        let query = req.uri.query().unwrap_or("");
        let query = serde_urlencoded::from_str(query);
        Box::pin(async move {
            match query {
                Ok(query) => Ok(Query(query)),
                Err(err) => Err(err.into()),
            }
        })
    }
}

impl<S, T> FromRequestParts<S> for Path<T>
where
    T: From<String> + Send + Sync + 'static,
{
    fn from_request_parts(req: &mut Parts, _state: Arc<S>) -> FRFut<Self> {
        let pp = req.extensions.get_mut::<PathParams>().unwrap();
        if pp.0.len() < 1 {
            return boxed_err(anyhow!("path params not long enough"));
        }
        let path = pp.0.remove(0).1.clone();
        Box::pin(async move { Ok(Path(path.into())) })
    }
}

impl<S: Send + Sync + 'static> FromRequestParts<S> for State<S> {
    fn from_request_parts(_req: &mut Parts, state: Arc<S>) -> FRPFut<'_, Self> {
        Box::pin(async move { Ok(State(state.clone())) })
    }
}

impl<S> FromRequest<S> for String {
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let body = req.body_mut().collect().await.unwrap().to_bytes();
            let string = std::str::from_utf8(&body)
                .map(|s| s.to_string())
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))?;
            Ok(string)
        })
    }
}

impl<S> FromRequest<S> for Bytes {
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let body = req.body_mut().collect().await.unwrap().to_bytes();
            Ok(body)
        })
    }
}

impl<S, T> FromRequest<S> for Form<T>
where
    T: DeserializeOwned + Send + Sync + 'static,
{
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let body = req.body_mut().collect().await?;
            let form: T = serde_urlencoded::from_bytes(&*body.to_bytes())?;
            Ok(Form(form))
        })
    }
}

impl<S> FromRequestParts<S> for Method {
    fn from_request_parts(req: &mut Parts, _: Arc<S>) -> FRPFut<'_, Self>
    where
        Self: Sized,
    {
        Box::pin(async move { Ok(req.method.clone()) })
    }
}

impl<S> FromRequestParts<S> for Extensions {
    fn from_request_parts(req: &mut Parts, _: Arc<S>) -> FRPFut<'_, Self>
    where
        Self: Sized,
    {
        Box::pin(async move { Ok(req.extensions.clone()) })
    }
}

impl<S> FromRequestParts<S> for Uri {
    fn from_request_parts(req: &mut Parts, _: Arc<S>) -> FRPFut<'_, Self>
    where
        Self: Sized,
    {
        Box::pin(async move { Ok(req.uri.clone()) })
    }
}
