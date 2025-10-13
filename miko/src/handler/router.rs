use std::{collections::HashMap, convert::Infallible, sync::Arc};
use std::path::PathBuf;
use crate::handler::handler::{handler_to_svc, DynHandler};
use crate::handler::nested_handler::NestLayer;
use crate::handler::router_svc::RouterSvc;
use crate::handler::{extractor::{from_request::FromRequest, path_params::PathParams}, handler::{FnOnceTuple, Req, Resp, TypedHandler}, into_response::IntoResponse};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Method, Request};
use matchit::Router as MRouter;
use miko_core::{encode_route, IntoMethods};
use tower::{util::BoxCloneService, Layer, Service};
#[cfg(feature = "ext")]
use crate::ext::static_svc::StaticSvcBuilder;

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
        ) as DynHandler;
        self.routes
            .entry(Method::$m)
            .or_insert_with(|| MRouter::new())
            .insert(encode_route(path), handler_to_svc(handler.clone()))
            .unwrap();
        self.path_map
          .entry(Method::$m)
          .or_insert_with(|| HashMap::new())
          .insert(path.to_string(), handler_to_svc(handler.clone()));
        self
      }
    }
}
pub type HttpReq = Request<Incoming>;
pub type HttpSvc<T = HttpReq> = BoxCloneService<T, Resp, Infallible>;

pub struct Router<S = ()> {
  pub routes: HashMap<Method, MRouter<HttpSvc<Req>>>,
  pub state: Arc<S>,
  pub layers: Vec<Arc<dyn Fn(HttpSvc<Req>) -> HttpSvc<Req> + Send + Sync>>,
  pub path_map: HashMap<Method, HashMap<String, HttpSvc<Req>>>
}
impl<S> Clone for Router<S> {
    fn clone(&self) -> Self {
        Self {
            routes: self.routes.clone(),
            state: self.state.clone(),
            layers: self.layers.clone(),
            path_map: self.path_map.clone(),
        }
    }
}

impl<S: Send + Sync + 'static> Router<S> {
    pub fn find_handler(&self, method: &Method, path: &str) -> Option<(HttpSvc<Req>, PathParams)> {
        if let Some(router) = self.routes.get(method) {
            match router.at(path) {
                Ok(matched) => {
                    let handler = matched.value.clone();
                    Some((handler, PathParams::from(&matched.params)))
                }
                Err(_e) => {
                    None
                }
            }
        } else {
            None
        }
    }
  pub async fn handle(&self, method: &Method, path: &str, mut req: Req) -> Resp {
    if let Some(router) = self.routes.get(method) {
      match router.at(path) {
        Ok(matched) => {
          req.extensions_mut().insert(PathParams::from(&matched.params));
          let mut handler = matched.value.clone();
          handler.call(req).await.map_err(|_| unreachable!()).unwrap().into_response()
        }
        Err(_e) => {
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

impl Router {
  pub fn new() -> Self {
    Self {
      routes: HashMap::new(),
      state: Arc::new(()),
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
    ) as DynHandler;
    for m in method.into_methods() {
      self.routes
        .entry(m.clone())
        .or_insert_with(|| MRouter::new())
        .insert(encode_route(path), handler_to_svc(handler.clone()))
        .unwrap();
      self.path_map
        .entry(m.clone())
        .or_insert_with(|| HashMap::new())
        .insert(path.to_string(), handler_to_svc(handler.clone()));
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

    pub fn nest<T>(&mut self, prefix: &str, mut other: Router<T>) -> &mut Self {
        let prefix = prefix.trim_end_matches('/').to_string();

        for (method, _) in other.routes.drain() {
            for (path, svc) in other.path_map.get_mut(&method).unwrap().drain() {
                let layered = NestLayer::new(&prefix).layer(svc);
                let boxed: HttpSvc<Req> = BoxCloneService::new(layered);
                let new_path = format!("{}{}", prefix, path);
                self.routes
                    .entry(method.clone())
                    .or_insert_with(|| MRouter::new())
                    .insert(&new_path, boxed.clone()).unwrap();
                self.path_map
                    .entry(method.clone())
                    .or_insert_with(|| HashMap::new())
                    .insert(new_path, boxed.clone());
            }
        }
        self
    }

    pub fn nest_service(&mut self, prefix: &str, svc: HttpSvc<Req>) {
        let prefix = prefix.trim_end_matches('/').to_string();
        let layered = NestLayer::new(&prefix).layer(svc);
        let boxed: HttpSvc<Req> = BoxCloneService::new(layered);
        let methods = [
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::HEAD,
            Method::OPTIONS,
        ];
        let new_path = format!("{}{}", prefix, "/{*rest}");
        for method in methods {
            self.routes
                .entry(method.clone())
                .or_insert_with(|| MRouter::new())
                .insert(&new_path, boxed.clone()).unwrap();
            self.path_map
                .entry(method.clone())
                .or_insert_with(|| HashMap::new())
                .insert(new_path.clone(), boxed.clone());
        }
    }

  pub fn with_layer<L>(mut self, layer: L) -> Self
  where
    L: Layer<HttpSvc<Req>> + Send + Sync + 'static,
    L::Service: Service<Req, Response = Resp, Error = Infallible> + Clone + Send + 'static,
    <L::Service as Service<Req>>::Future: Send + 'static,
  {
    self.layers.push(Arc::new(move |svc: HttpSvc<Req>| {
        let wrapped = layer.layer(svc);
        BoxCloneService::new(wrapped)
    }));
    self
  }

  pub fn into_tower_service(mut self) -> HttpSvc<Req> {
    let layers = std::mem::take(&mut self.layers);
    let mut svc: HttpSvc<Req> = BoxCloneService::new(RouterSvc {router: self });
    for apply in layers {
        svc = apply(svc);
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

#[cfg(feature = "ext")]
impl<S: Send + Sync + 'static> Router<S> {
    pub fn static_svc<F>(&mut self, prefix: &str, root: impl Into<PathBuf>, option_closure: Option<F>)
    where F: FnOnce(StaticSvcBuilder) -> StaticSvcBuilder
    {
        let builder = StaticSvcBuilder::new(root);
        let builder = if let Some(option_closure) = option_closure {
            option_closure(builder)
        } else {
            builder
        };
        self.nest_service(prefix, builder.build())
    }
}