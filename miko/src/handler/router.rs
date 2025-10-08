use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Method;
use matchit::Router as MRouter;

use crate::handler::{extractor::from_request::FromRequest, handler::{FnOnceTuple, Handler, Req, Resp, TypedHandler}, into_response::IntoResponse};

pub struct Router<S = ()> {
  pub routes: HashMap<Method, MRouter<Arc<dyn Handler>>>,
  state: Arc<S>,
}

impl<S: Send + Sync + 'static> Router<S> {
  pub async fn handle(&self, method: Method, path: &str, req: Req) -> Resp {
    if let Some(handler) = self.routes.get(&method).and_then(|router| {
      router.at(path).map(|node| {
        node.value.clone()
      }).ok()
    }) {
      handler.call(req).await.into_response()
    } else {
      let res = hyper::Response::builder()
        .status(hyper::StatusCode::NOT_FOUND)
        .body(
          Full::new(Bytes::from("Not Found")).boxed()
        )
        .unwrap();
      res
    }
  }
}

impl Router {
  pub fn new() -> Self {
    Self { routes: HashMap::new(), state: Arc::new(()) }
  }
}

impl<S: Send + Sync + 'static> Router<S> {
  pub fn route<F, A, Fut, R>(&mut self, method: &[Method], path: &str, handler: F)
  where
    F: FnOnceTuple<A, Output = Fut> + Clone + Send + Sync + 'static,
    A: FromRequest<S> + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
  {
    let handler = Arc::new(
      TypedHandler::new(handler, self.state.clone())
    ) as Arc<dyn Handler>;
    for m in method {
      self.routes
        .entry(m.clone())
        .or_insert_with(|| MRouter::new())
        .insert(path, handler.clone())
        .unwrap();
    }
  }
}
