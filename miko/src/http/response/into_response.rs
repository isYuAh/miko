use crate::extractor::Json;
use crate::handler::{Resp, RespBody};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::HeaderMap;
use hyper::{Response, StatusCode, body::Frame};
use serde::Serialize;

/// 将一个类型转换为 HTTP 响应的通用能力
///
/// 你的 handler 返回值只要实现了该 trait，就可以被框架自动转换为响应。
/// 框架已为 String、&str、Json<T>、Result、()、(StatusCode, T) 等常见类型提供实现。
pub trait IntoResponse {
    fn into_response(self) -> Resp;
}

fn bytes_to_boxed(bytes: Bytes) -> RespBody {
    Full::new(bytes).boxed()
}

impl IntoResponse for String {
    fn into_response(self) -> Resp {
        Response::builder()
            .status(200)
            .header("content-type", "text/plain; charset=utf-8")
            .body(bytes_to_boxed(Bytes::from(self)))
            .unwrap()
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Resp {
        Response::builder()
            .status(200)
            .header("content-type", "text/plain; charset=utf-8")
            .body(bytes_to_boxed(Bytes::from(self)))
            .unwrap()
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Resp {
        let body = serde_json::to_vec(&self.0).unwrap();
        Response::builder()
            .header("content-type", "application/json")
            .body(bytes_to_boxed(Bytes::from(body)))
            .unwrap()
    }
}

/// HTML 响应包装器
///
/// 用于返回 HTML 内容，自动设置 content-type 为 text/html
///
/// # Example
/// ```no_run
/// use miko::http::response::into_response::{IntoResponse, Html};
///
/// async fn handler() -> impl IntoResponse {
///     Html("<h1>Hello World</h1>".to_string())
/// }
/// ```
pub struct Html(pub String);

impl IntoResponse for Html {
    fn into_response(self) -> Resp {
        Response::builder()
            .header("content-type", "text/html; charset=utf-8")
            .body(bytes_to_boxed(Bytes::from(self.0)))
            .unwrap()
    }
}

impl IntoResponse for Resp {
    fn into_response(self) -> Resp {
        self
    }
}

/// SSE 响应包装器，将一个字节流包装为 text/event-stream 响应
pub struct SSE<T>(pub T);

impl<S, E> IntoResponse for SSE<S>
where
    S: Stream<Item = Result<Bytes, E>> + Send + Sync + 'static,
    E: std::fmt::Debug + Send + 'static,
{
    fn into_response(self) -> Resp {
        let body = BodyExt::boxed(StreamBody::new(self.0.map(|chunk| match chunk {
            Ok(b) => Ok(Frame::data(b)),
            Err(e) => {
                tracing::error!("SSE stream error: {:?}", e);
                Ok(Frame::data(Bytes::from_static(b"\n")))
            }
        })));
        Response::builder()
            .status(200)
            .header("content-type", "text/event-stream")
            .body(body)
            .unwrap()
    }
}

impl IntoResponse for anyhow::Error {
    fn into_response(self) -> Resp {
        use crate::error::AppError;
        let app_error: AppError = self.into();
        app_error.into_response()
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: IntoResponse,
{
    fn into_response(self) -> Resp {
        match self {
            Ok(t) => t.into_response(),
            Err(e) => e.into_response(),
        }
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Resp {
        Response::builder()
            .status(200)
            .body(miko_core::fast_builder::box_empty_body())
            .unwrap()
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Resp {
        Response::builder()
            .status(self)
            .body(miko_core::fast_builder::box_empty_body())
            .unwrap()
    }
}

impl<T> IntoResponse for (StatusCode, T)
where
    T: IntoResponse,
{
    fn into_response(self) -> Resp {
        let body = self.1.into_response();
        Response::builder()
            .status(self.0)
            .body(body.into_body())
            .unwrap()
    }
}

impl<T> IntoResponse for (HeaderMap, T)
where
    T: IntoResponse,
{
    fn into_response(self) -> Resp {
        let mut response = self.1.into_response();
        let h = response.headers_mut();
        for (name, value) in self.0 {
            if name.is_none() {
                continue;
            }
            h.insert(name.unwrap().clone(), value);
        }
        response
    }
}

impl<T> IntoResponse for (StatusCode, HeaderMap, T)
where
    T: IntoResponse,
{
    fn into_response(self) -> Resp {
        let mut response = self.2.into_response();
        let h = response.headers_mut();
        for (name, value) in self.1 {
            if name.is_none() {
                continue;
            }
            h.insert(name.unwrap().clone(), value);
        }
        *response.status_mut() = self.0;
        response
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Resp {
        Response::builder()
            .header("content-type", "application/octet-stream")
            .body(bytes_to_boxed(Bytes::from(self)))
            .unwrap()
    }
}

impl IntoResponse for serde_json::Value {
    fn into_response(self) -> Resp {
        let body = serde_json::to_vec(&self).unwrap();
        Response::builder()
            .header("content-type", "application/json")
            .body(bytes_to_boxed(Bytes::from(body)))
            .unwrap()
    }
}
