use std::{sync::Arc};
use http_body_util::BodyExt;
use hyper::http::request::Parts;
use serde::de::DeserializeOwned;
use crate::handler::{extractor::from_request::{FRFut, FromRequest, FromRequestParts}, handler::{Req}};

pub struct Json<T>(pub T);
pub struct Query<T>(pub T);
pub struct Path<T>(pub T);
pub struct State<T>(pub T);

impl<T> FromRequest for Json<T>
where T: DeserializeOwned + Send + Sync + 'static {
    fn from_request(mut req: Req, _state: Arc<()>) -> FRFut<Self> {
        let _ = _state;
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

impl<T: Clone> FromRequestParts<T> for State<T>
where T: DeserializeOwned + Send + Sync + 'static {
    fn from_request_parts(_: &Parts, _state: Arc<T>) -> FRFut<Self> {
        Box::pin(async move {
            State(_state.as_ref().clone())
        })
    }
}