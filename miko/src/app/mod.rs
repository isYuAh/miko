use crate::handler::handler::Req;
use crate::http::convert::incoming_to_req::IncomingToInternal;
use crate::router::HttpSvc;
use crate::router::Router;
use config::ApplicationConfig;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as AutoBuilder,
    service::TowerToHyperService,
};
use std::sync::Arc;
use tokio::io::Result as IoResult;
use tokio::net::TcpListener;
use tracing;

pub mod config;

/// 应用程序入口，负责持有配置与路由，并启动 HTTP 服务
pub struct Application {
    config: ApplicationConfig,
    svc: HttpSvc<Req>,
}

/// 应用程序
impl Application {
    /// 使用给定的配置与 Router 构建一个应用实例
    ///
    /// 一般情况下，你可以使用 [`Application::new_`] 读取默认配置后创建。
    pub fn new<S: Send + Sync + 'static>(
        config: ApplicationConfig,
        router: Router<S>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            svc: router.into_tower_service(),
        })
    }

    /// 使用默认/合并后的配置与 Router 构建应用实例
    ///
    /// 该方法会读取项目根目录下的配置文件，失败时会回退到内置默认值。
    pub fn new_<S: Send + Sync + 'static>(router: Router<S>) -> Arc<Self> {
        Self::new(ApplicationConfig::load_().unwrap_or_default(), router)
    }
}

impl Application {
    /// 运行应用，基于配置中的地址与端口监听并处理请求
    ///
    /// 此方法会阻塞当前异步任务，直到出现网络错误或手动终止。
    pub async fn run(self: Arc<Self>) -> IoResult<()> {
        let addr = format!("{}:{}", self.config.addr, self.config.port);
        let listener = TcpListener::bind(addr).await?;
        let executor = TokioExecutor::new();
        let service = self.svc.clone();
        tracing::info!("listening on {}:{}", self.config.addr, self.config.port);
        loop {
            let builder = AutoBuilder::new(executor.clone());
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let service = TowerToHyperService::new(IncomingToInternal {
                inner: service.clone(),
            });
            tokio::spawn(async move {
                if let Err(_e) = builder.serve_connection_with_upgrades(io, service).await {
                    if let Some(hyper_err) = _e.downcast_ref::<hyper::Error>() {
                        if hyper_err.is_incomplete_message() {
                            return;
                        }
                    }
                    tracing::warn!("conn error {_e}");
                };
            });
        }
    }
}
