use std::{collections::HashMap, convert::Infallible, ops::Deref, pin::Pin, sync::Arc, task::{Context, Poll}};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Method, Request};
use matchit::Router as MRouter;
use miko_core::{encode_route, IntoMethods};
use tower::{util::BoxCloneService, Layer, Service};

use crate::handler::{extractor::{from_request::FromRequest, path_params::PathParams}, handler::{FnOnceTuple, Handler, Req, Resp, TypedHandler}, into_response::IntoResponse};

macro_rules! define_method {
    ($name:ident, $m:ident) => {
      pub fn $name<F, A, Fut, R, M>(&mut self, path: &str, handler: F) -> &mut Self
      where
        F: FnOnceTuple<A, Output = Fut> + Clone + Send + Sync + 'static,
        A: FromRequest<S, M> + Send + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
        M: Send + Sync + 'static,
      {
        let handler = Arc::new(
          TypedHandler::new(handler, self.state.clone())
        ) as Arc<dyn Handler>;
        self.routes
            .entry(Method::$m)
            .or_insert_with(|| MRouter::new())
            .insert(encode_route(path), handler.clone())
            .unwrap();
        self.path_map
          .entry(Method::$m)
          .or_insert_with(|| HashMap::new())
          .insert(path.to_string(), handler.clone());
        self
      }
    }
}
type HttpReq = Request<Incoming>;
type HttpSvc = BoxCloneService<HttpReq, Resp, Infallible>;
// #[derive(Clone)]
pub struct Router<S = ()> {
  pub routes: HashMap<Method, MRouter<Arc<dyn Handler>>>,
  pub state: Arc<S>,
  layers: Vec<Box<dyn Fn(HttpSvc) -> HttpSvc + Send + Sync>>,
  path_map: HashMap<Method, HashMap<String, Arc<dyn Handler>>>
}

impl<S: Send + Sync + 'static> Router<S> {
  pub async fn handle(&self, method: &Method, path: &str, mut req: Req) -> Resp {
    if let Some(router) = self.routes.get(method) {
      match router.at(path) {
        Ok(matched) => {
          req.extensions_mut().insert(PathParams::from(&matched.params));
          let handler = matched.value.clone();
          handler.call(req).await.into_response()
        }
        Err(_) => {
          hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")).boxed())
            .unwrap()
        }
      }
    } else {
      hyper::Response::builder()
        .status(hyper::StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")).boxed())
        .unwrap()
    }
  }
}

pub struct ArcRouter<S>(pub Arc<Router<S>>);
impl<S> Deref for ArcRouter<S> {
  type Target = Router<S>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
impl<S> Clone for ArcRouter<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Router {
  pub fn new() -> Self {
    Self {
      routes: HashMap::new(),
      state: Arc::new(()),
      // svc_builder: ServiceBuilder::new()
      layers: Vec::new(),
      path_map: HashMap::new(),
    }
  }
}


impl<S: Send + Sync + 'static> Router<S> {
  pub fn route<F, A, Fut, R, M>(&mut self, method: impl IntoMethods, path: &str, handler: F) -> &mut Self
  where
    F: FnOnceTuple<A, Output = Fut> + Clone + Send + Sync + 'static,
    A: FromRequest<S, M> + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
    M: Send + Sync + 'static,
  {
    let handler = Arc::new(
      TypedHandler::new(handler, self.state.clone())
    ) as Arc<dyn Handler>;
    for m in method.into_methods() {
      self.routes
        .entry(m.clone())
        .or_insert_with(|| MRouter::new())
        .insert(encode_route(path), handler.clone())
        .unwrap();
      self.path_map
        .entry(m.clone())
        .or_insert_with(|| HashMap::new())
        .insert(path.to_string(), handler.clone());
    }
    self
  }

  define_method!(get, GET);
  define_method!(post, POST);
  define_method!(put, PUT);
  define_method!(delete, DELETE);
  define_method!(head, HEAD);
  define_method!(options, OPTIONS);
  define_method!(trace, TRACE);
  define_method!(connect, CONNECT);
  define_method!(patch, PATCH);
}

impl<S: Send + Sync + 'static> tower::Service<Request<Incoming>> for ArcRouter<S> {
    type Response = Resp;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Resp, Infallible>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req_incoming: Request<Incoming>) -> Self::Future {
        let router = self.0.clone();
        Box::pin(async move {
            let method = &req_incoming.method().clone();
            let path = &req_incoming.uri().path().to_string();

            let req: Req = req_incoming
                .map(|inc| inc.map_err(|_| unreachable!()).boxed());

            let resp = router.handle(method, path, req).await;
            Ok(resp)
        })
    }
}

impl<S: Send + Sync + 'static> Router<S> {
  pub fn with_state<T>(self, state: T) -> Router<T> {
    Router { routes: self.routes, state: Arc::new(state), layers: self.layers, path_map: self.path_map }
  }

  pub fn merge(&mut self, other: Self) -> &mut Self {
    for (method, router) in other.routes {
      self.routes
        .entry(method.clone())
        .or_insert_with(|| MRouter::new())
        .merge(router)
        .unwrap();
      self.path_map
        .entry(method.clone())
        .or_insert_with(|| HashMap::new())
        .extend(other.path_map.get(&method).unwrap().iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    self
  }

  pub fn nest(&mut self, prefix: &str, mut other: Router<S>) -> &mut Self {
      let prefix = prefix.trim_end_matches('/').to_string();

      for (method, _) in other.routes.drain() {
          for (path, handler) in other.path_map.get(&method).unwrap().iter() {
              let new_path = format!("{}{}", prefix, path);
              self.routes
                .entry(method.clone())
                .or_insert_with(|| MRouter::new())
                .insert(&new_path, handler.clone()).unwrap();
              self.path_map
                .entry(method.clone())
                .or_insert_with(|| HashMap::new())
                .insert(new_path, handler.clone());
          }
      }
      self
  }

  pub fn with_service<L>(mut self, layer: L) -> Self
  where
    L: Layer<HttpSvc> + Send + Sync + 'static,
    L::Service: Service<HttpReq, Response = Resp, Error = Infallible> + Clone + Send + 'static,
    <L::Service as Service<HttpReq>>::Future: Send + 'static,
  {
    self.layers.push(Box::new(move |svc: HttpSvc| {
        let wrapped = layer.layer(svc);
        BoxCloneService::new(wrapped)
    }));
    self
  }

  pub fn into_tower_service(mut self) -> HttpSvc {
    let layers = std::mem::take(&mut self.layers);
    let mut svc: HttpSvc = BoxCloneService::new(ArcRouter(Arc::new(self)));
    for apply in layers {
        svc = (apply)(svc);
    }
    svc
  }

  pub fn take(&mut self) -> Self {
    std::mem::replace(self, Router {
      routes: HashMap::new(),
      state: self.state.clone(),
      layers: Vec::new(),
      path_map: HashMap::new(),
    })
  }
}