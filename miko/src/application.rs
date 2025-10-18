use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as AutoBuilder,
    service::TowerToHyperService,
};
use std::sync::Arc;
use tokio::io::Result as IoResult;
use tokio::net::TcpListener;
#[cfg(feature = "inner_log")]
use tracing;

use crate::handler::handler::Req;
use crate::handler::incoming_to_req::IncomingToInternal;
use crate::handler::router::HttpSvc;
use crate::{config::config::ApplicationConfig, handler::router::Router};

pub struct Application {
    config: ApplicationConfig,
    svc: HttpSvc<Req>,
}

/// 应用程序
impl Application {
    pub fn new<S: Send + Sync + 'static>(
        config: ApplicationConfig,
        router: Router<S>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            svc: router.into_tower_service(),
        })
    }

    pub fn new_<S: Send + Sync + 'static>(router: Router<S>) -> Arc<Self> {
        Self::new(ApplicationConfig::load_().unwrap_or_default(), router)
    }
}

impl Application {
    /// 运行一个应用程序，基于self.config和self.router开始监听
    pub async fn run(self: Arc<Self>) -> IoResult<()> {
        let addr = format!("{}:{}", self.config.addr, self.config.port);
        let listener = TcpListener::bind(addr).await?;
        let executor = TokioExecutor::new();
        let service = self.svc.clone();
        #[cfg(feature = "inner_log")]
        tracing::info!("listening on {}:{}", self.config.addr, self.config.port);
        loop {
            let builder = AutoBuilder::new(executor.clone());
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let service = TowerToHyperService::new(IncomingToInternal {
                inner: service.clone(),
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
