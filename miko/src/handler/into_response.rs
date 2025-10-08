use bytes::Bytes;
use futures::{Stream, StreamExt};
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::{body::Frame, Response};
use serde::Serialize;

use crate::handler::handler::{Resp, RespBody};
pub trait IntoResponse {
  fn into_response(self) -> Resp;
}

fn bytes_to_boxed(bytes: Bytes) -> RespBody {
  Full::new(bytes).boxed()
}

impl IntoResponse for String {
  fn into_response(self) -> Resp {
      Response::new(bytes_to_boxed(Bytes::from(self)))
  }
}

impl IntoResponse for &'static str {
  fn into_response(self) -> Resp {
      Response::new(bytes_to_boxed(Bytes::from(self)))
  }
}

pub struct Json<T>(pub T);

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

// impl<S, E> IntoResponse for StreamBody<S>
// where S: Stream<Item = Result<Bytes, E>> + Send + 'static,
//       E: std::fmt::Debug + Send + 'static,
// {
//     fn into_response(self) -> Resp {
//       let body = StreamBody::new(self);
//       Response::builder()
//         .status(200)
//         .header("content-type", "application/octet-stream")
//         .body(self.boxed())
//         .unwrap()
//     }
// }