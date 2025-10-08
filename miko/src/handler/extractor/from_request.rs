use std::sync::Arc;

use http_body_util::{BodyExt};
use hyper::{HeaderMap};
use hyper::http::request::Parts;
use serde::{de::DeserializeOwned};

use crate::handler::handler::{PartsTag, ReqTag, Which};
use crate::handler::{extractor::extractors::{Query, Json, Path}, handler::Req};
pub type FRFut<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'static>>;
pub type FRPFut<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
pub trait FromRequest<S = ()>: Send + Sync + 'static {
  fn from_request(req: Req, state: Arc<S>) -> FRFut<Self>;
}
pub trait FromRequestParts<S = ()>: Send + Sync + 'static {
  fn from_request_parts<'a>(req: &'a Parts, state: Arc<S>) -> FRPFut<'a, Self>;
}

impl FromRequest for Req {
    fn from_request(req: Req, _state: Arc<()>) -> FRFut<Self> {
      Box::pin(async move {
          req
      })
    }
}
impl<T> FromRequest for Json<T>
where T: DeserializeOwned + Send + Sync + 'static {
    fn from_request(mut req: Req, _state: Arc<()>) -> FRFut<Self> {
        Box::pin(async move {
            let body = req.body_mut().collect().await.unwrap().to_bytes();
            let json = serde_json::from_slice(&body).unwrap();
            Json(json)
        })
    }
}


impl<T> FromRequestParts for Query<T>
where T: DeserializeOwned + Send + Sync + 'static {
    fn from_request_parts(req: &Parts, _state: Arc<()>) -> FRFut<Self> {
        let query = req.uri.query().unwrap_or("");
        let query = serde_urlencoded::from_str(query).unwrap();
        Box::pin(async move {
            Query(query)
        })
    }
}

impl FromRequestParts for HeaderMap {
    fn from_request_parts(req: &Parts, _state: Arc<()>) -> FRFut<Self> {
        let headers = req.headers.clone();
        Box::pin(async move {
            headers 
        })
    }
}


impl Which for Req {
    type Tag = ReqTag;
}

impl<T> Which for Json<T> {
    type Tag = ReqTag;
}

impl<T> Which for Query<T> {
    type Tag = PartsTag;
}

impl Which for HeaderMap {
    type Tag = PartsTag;
}

impl<T> Which for Path<T> {
    type Tag = PartsTag;
}