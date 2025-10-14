use bytes::Bytes;
use futures::{Stream, StreamExt};
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::{body::Frame, Response, StatusCode};
use serde::Serialize;
use miko_core::fast_builder::ResponseBuilder;
use crate::handler::extractor::extractors::Json;
use crate::handler::handler::{Resp, RespBody};
pub trait IntoResponse {
  fn into_response(self) -> Resp;
}

fn bytes_to_boxed(bytes: Bytes) -> RespBody {
  Full::new(bytes).boxed()
}

impl IntoResponse for String {
  fn into_response(self) -> Resp {
      ResponseBuilder::ok(self).unwrap()
  }
}

impl IntoResponse for &'static str {
  fn into_response(self) -> Resp {
      ResponseBuilder::ok(self.to_string()).unwrap()
  }
}

impl<T: Serialize> IntoResponse for Json<T> {
  fn into_response(self) -> Resp {
    let body = serde_json::to_vec(&self.0).unwrap();
    Response::builder().header("content-type", "application/json")
        .body(bytes_to_boxed(Bytes::from(body))).unwrap()
  }
}

impl IntoResponse for Resp {
  fn into_response(self) -> Resp {
    self
  }
}

pub struct SSE<T>(pub T);

impl<S, E> IntoResponse for SSE<S>
where
  S: Stream<Item = Result<Bytes, E>> + Send + Sync + 'static,
  E: std::fmt::Debug + Send + 'static,
{
  fn into_response(self) -> Resp {
    let body = BodyExt::boxed(StreamBody::new(
      self.0.map(|chunk| match chunk {
        Ok(b) => Ok(Frame::data(b)),
        Err(e) => {
          tracing::error!("SSE stream error: {:?}", e);
          Ok(Frame::data(Bytes::from_static(b"\n")))
        }
      })
    ));
    Response::builder()
      .status(200)
      .header("content-type", "text/event-stream")
      .body(body)
      .unwrap()
  }
}

impl IntoResponse for anyhow::Error {
  fn into_response(self) -> Resp {
    #[cfg(feature = "inner_log")]
    tracing::error!("{}", self);
    Response::builder()
      .status(500)
      .header("content-type", "text/plain;charset=utf-8")
      .body(bytes_to_boxed(Bytes::from(format!("Internal Server Error: {}", self))))
      .unwrap()
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
    ResponseBuilder::ok("".to_string()).unwrap()
  }
}

impl<T> IntoResponse for (StatusCode, T)
where
    T: IntoResponse
{
  fn into_response(self) -> Resp {
    let body = self.1.into_response();
    Response::builder()
      .status(self.0)
      .body(body.into_body())
      .unwrap()
  }
}