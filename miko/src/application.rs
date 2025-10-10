use std::{convert::Infallible, sync::Arc};
use http_body_util::BodyExt;
use hyper::{body::Incoming, service::service_fn, Request, server::conn::http1};
use hyper_util::{rt::{TokioExecutor, TokioIo}, server::conn::auto::Builder as AutoBuilder, service::TowerToHyperService};
use tokio::net::{TcpListener, TcpStream};
#[cfg(feature = "inner_log")]
use tracing;
use tokio::io::Result as IoResult;

use crate::{config::config::ApplicationConfig, handler::{into_response::IntoResponse, router::{ArcRouter, Router}}};
pub struct Application<S = ()> {
  config: ApplicationConfig,
  router: ArcRouter<S>,
}

impl<S> Application<S> {
  pub fn new(config: ApplicationConfig, router: Router<S>) -> Arc<Self> {
    Arc::new(Self { config, router: ArcRouter(Arc::new(router)) })
  }

  pub fn new_(router: Router<S>) -> Arc<Self> {
    Self::new(ApplicationConfig::load_().unwrap_or_default(), router)
  }
}

impl<S: Send + Sync + 'static> Application<S> {
  pub async fn handle_connection(self: Arc<Self>, stream: TcpStream) -> IoResult<()> {
    let io = TokioIo::new(stream);
    let router = self.router.0.clone();
    let service = service_fn(move |req: Request<Incoming>| {
      let router = router.clone();
      async move {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let body_boxed = req.map(|b| b.map_err(|_| unreachable!()).boxed());
        let resp = router.handle(&method, &path, body_boxed).await;
        Ok::<_, Infallible>(resp.into_response())
      }
    });
    http1::Builder::new()
      .serve_connection(io, service)
      .await
      .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::Other, e))?;
    Ok(())
  }

  pub async fn run(self: Arc<Self>) -> IoResult<()> {
    let addr = format!("{}:{}", self.config.addr, self.config.port);
    let listener = TcpListener::bind(addr).await?;
    let executor = TokioExecutor::new();
    let service = self.router.clone();
    #[cfg(feature = "inner_log")]
    tracing::info!("listening on {}", self.config.addr);
    loop {
      let builder = AutoBuilder::new(executor.clone());
      let (stream, _) = listener.accept().await?;
      let io = TokioIo::new(stream);
      let service = service.clone();
      let service = TowerToHyperService::new(service);
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