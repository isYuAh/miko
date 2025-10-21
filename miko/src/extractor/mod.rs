pub mod from_request;
pub mod multipart;
pub mod path_params;

#[cfg(feature = "validation")]
pub mod validated_json;

#[cfg(feature = "validation")]
pub use validated_json::ValidatedJson;

use crate::error::AppError;
use crate::extractor::from_request::FRPFut;
use crate::extractor::from_request::{FRFut, FromRequest, FromRequestParts};
use crate::extractor::path_params::PathParams;
use crate::handler::handler::Req;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::http::Extensions;
use hyper::http::request::Parts;
use hyper::{Method, Uri};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::sync::Arc;

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
            let body = req
                .body_mut()
                .collect()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read request body: {}", e)))?
                .to_bytes();

            // 直接使用 JsonParseError，包含原始的 serde_json::Error
            let json =
                serde_json::from_slice::<T>(&body).map_err(|e| AppError::JsonParseError(e))?;

            Ok(Json(json))
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
            query
                .map(Query)
                .map_err(|e| AppError::UrlEncodedParseError(e))
        })
    }
}

impl<S, T> FromRequestParts<S> for Path<T>
where
    T: std::str::FromStr + Send + Sync + 'static,
    T::Err: std::fmt::Display,
{
    fn from_request_parts(req: &mut Parts, _state: Arc<S>) -> FRFut<Self> {
        let pp = req.extensions.get_mut::<PathParams>().unwrap();
        if pp.0.len() < 1 {
            return Box::pin(async move {
                Err(AppError::BadRequest("No path parameters found".to_string()))
            });
        }
        let path = pp.0.remove(0).1.clone();
        Box::pin(async move {
            match path.parse::<T>() {
                Ok(value) => Ok(Path(value)),
                Err(err) => Err(AppError::BadRequest(format!(
                    "Failed to parse path parameter '{}': {}",
                    path, err
                ))),
            }
        })
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
            let body = req
                .body_mut()
                .collect()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read request body: {}", e)))?
                .to_bytes();
            let string = std::str::from_utf8(&body)
                .map(|s| s.to_string())
                .map_err(|e| {
                    AppError::BadRequest(format!("Invalid UTF-8 in request body: {}", e))
                })?;
            Ok(string)
        })
    }
}

impl<S> FromRequest<S> for Bytes {
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let body = req
                .body_mut()
                .collect()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read request body: {}", e)))?
                .to_bytes();
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
            let body = req
                .body_mut()
                .collect()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read request body: {}", e)))?
                .to_bytes();
            let form: T = serde_urlencoded::from_bytes(&body)
                .map_err(|e| AppError::UrlEncodedParseError(e))?;
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
