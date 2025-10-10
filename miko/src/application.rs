use std::{convert::Infallible, sync::Arc};
use http_body_util::BodyExt;
use hyper::{body::Incoming, service::service_fn, Request, server::conn::http1};
use hyper_util::{rt::{TokioExecutor, TokioIo}, server::conn::auto::Builder as AutoBuilder, service::TowerToHyperService};
use tokio::net::{TcpListener, TcpStream};
#[cfg(feature = "inner_log")]
use tracing;
use tokio::io::Result as IoResult;

use crate::{config::config::ApplicationConfig, handler::{into_response::IntoResponse, router::{Router}}};
use crate::handler::handler::Req;
use crate::handler::incoming_to_req::IncomingToInternal;
use crate::handler::router::HttpSvc;

pub struct Application {
  config: ApplicationConfig,
  svc: HttpSvc<Req>,
}

impl Application {
  pub fn new<S: Send + Sync + 'static>(config: ApplicationConfig, router: Router<S>) -> Arc<Self> {
    Arc::new(Self { config, svc: router.into_tower_service() })
  }

  pub fn new_<S: Send + Sync + 'static>(router: Router<S>) -> Arc<Self> {
    Self::new(ApplicationConfig::load_().unwrap_or_default(), router)
  }
}

impl Application {
  pub async fn run(self: Arc<Self>) -> IoResult<()> {
    let addr = format!("{}:{}", self.config.addr, self.config.port);
    let listener = TcpListener::bind(addr).await?;
    let executor = TokioExecutor::new();
    let service = self.svc.clone();
    #[cfg(feature = "inner_log")]
    tracing::info!("listening on {}", self.config.addr);
    loop {
      let builder = AutoBuilder::new(executor.clone());
      let (stream, _) = listener.accept().await?;
      let io = TokioIo::new(stream);
      let service = TowerToHyperService::new(IncomingToInternal {
        inner: service.clone()
      });
      tokio::spawn(async move {
        if let Err(_e) = builder.serve_connection(io, service).await {
          #[cfg(feature = "inner_log")]
          tracing::warn!("conn error {_e}");
        };
      });
    }
  }
}

// impl<S: Send + Sync + 'static> Application<S> {
//   pub fn route<F, A, Fut, R>(&mut self, method: Method, path: &str, handler: F)
//   where
//     F: FnOnceTuple<A, Output = Fut> + Clone + Send + Sync + 'static,
//     A: FromRequest<S> + Send + 'static,
//     Fut: Future<Output = R> + Send + 'static,
//     R: IntoResponse + Send + 'static,
//   {
//     self.router.route(method.into(), path, handler);
//   }
// }