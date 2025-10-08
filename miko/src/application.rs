use std::{convert::Infallible, sync::Arc};

use futures::io;
use http_body_util::BodyExt;
use hyper::{body::Incoming, service::service_fn, Request, server::conn::http1};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tracing;
use tokio::io::Result as IoResult;

use crate::{config::config::ApplicationConfig, handler::{extractor::from_request::FromRequest, handler::{FnOnceTuple, Resp}, into_response::IntoResponse, router::Router}};
pub struct Application<S = ()> {
  config: ApplicationConfig,
  router: Arc<Router<S>>,
}

impl<S> Application<S> {
  pub fn new(config: ApplicationConfig, router: Router<S>) -> Arc<Self> {
    Arc::new(Self { config, router: Arc::new(router) })
  }
}

impl<S: Send + Sync + 'static> Application<S> {
  pub async fn handle_connection(self: Arc<Self>, stream: TcpStream) -> IoResult<()> {
    let io = TokioIo::new(stream);
    let router = self.router.clone();
    let service = service_fn(move |req: Request<Incoming>| {
      let router = router.clone();
      async move {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let body_boxed = req.map(|b| b.map_err(|_| unreachable!()).boxed());
        let resp = router.handle(method, &path, body_boxed).await;
        Ok::<_, Infallible>(resp.into_response())
      }
    });
    http1::Builder::new()
      .serve_connection(io, service)
      .await
      .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::Other, e))?;
    Ok(())
  }

  pub async fn serve(self: Arc<Self>, io: TcpListener) -> IoResult<()> {
    tracing::info!("listening on {}", self.config.addr);
    loop {
      let (stream, _) = io.accept().await?;
      let app = self.clone();
      tokio::spawn(async move {
        if let Err(e) = app.handle_connection(stream).await {
          tracing::warn!("conn error {e}");
        };
      });
    }
  }

  pub async fn run(self: Arc<Self>) -> IoResult<()> {
    let io = TcpListener::bind(&self.config.addr).await?;
    self.serve(io).await?;
    Ok(())
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