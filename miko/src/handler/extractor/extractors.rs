use std::{sync::Arc};
use http_body_util::BodyExt;
use hyper::http::request::Parts;
use serde::de::DeserializeOwned;
use crate::handler::{extractor::from_request::{FRFut, FromRequest, FromRequestParts}, handler::{Req}};
use crate::handler::extractor::from_request::FRPFut;
use crate::handler::extractor::path_params::PathParams;

pub struct Json<T>(pub T);
pub struct Query<T>(pub T);
pub struct Path<T>(pub T);
pub struct State<T>(pub Arc<T>);

impl<T> FromRequest for Json<T>
where T: DeserializeOwned + Send + Sync + 'static {
    fn from_request(mut req: Req, _state: Arc<()>) -> FRFut<Self> {
        let _ = _state;
        Box::pin(async move {
            let body = req.body_mut().collect().await.unwrap().to_bytes();
            let json = serde_json::from_slice(&body);
            match json {
                Ok(json) => Ok(Json(json)),
                Err(err) => Err(err.into())
            }
        })
    }
}


impl<T> FromRequestParts for Query<T>
where T: DeserializeOwned + Send + Sync + 'static {
    fn from_request_parts(req: &mut Parts, _state: Arc<()>) -> FRFut<Self> {
        let query = req.uri.query().unwrap_or("");
        let query = serde_urlencoded::from_str(query);
        Box::pin(async move {
            match query {
                Ok(query) => Ok(Query(query)),
                Err(err) => Err(err.into())
            }
        })
    }
}

impl<T> FromRequestParts for Path<T>
where T: From<String> + Send + Sync + 'static
{
    fn from_request_parts(req: &mut Parts, _state: Arc<()>) -> FRFut<Self> {
        let pp = req.extensions.get_mut::<PathParams>().unwrap();
        let path = pp.0.remove(0).1.clone();
        Box::pin(async move {
            Ok(Path(path.into()))
        })
    }
}

impl<S: Send + Sync + 'static> FromRequestParts<S> for State<S> {
    fn from_request_parts(_req: &mut Parts, state: Arc<S>) -> FRPFut<Self> {
        Box::pin(async move {
            Ok(State(state.clone()))
        })
    }
}