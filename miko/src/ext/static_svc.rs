use crate::router::HttpSvc;
use http_body_util::BodyExt;
use hyper::{Method, Response};
use miko_core::fallible_stream_body::FallibleStreamBody;
use miko_core::fast_builder::ResponseBuilder;
use miko_core::{Req, Resp, decode_path};
use std::convert::Infallible;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tower::util::BoxCloneService;
use tower::{Layer, Service};
use tower_http::cors::CorsLayer;

/// 静态文件服务，实现目录下文件的按路径映射与可选 SPA 回退
#[derive(Clone)]
pub struct StaticSvc {
    pub root: Arc<PathBuf>,
    pub spa_fallback: bool,
}
impl StaticSvc {
    /// 构建一个静态服务的 Builder
    pub fn builder(root: impl Into<PathBuf>) -> StaticSvcBuilder {
        StaticSvcBuilder::new(root)
    }
    fn resolve_path(&self, uri_path: &str) -> PathBuf {
        let mut path = self.root.as_ref().clone();
        let decoded = decode_path(uri_path);
        let safe_rel = Path::new(&decoded)
            .components()
            .filter(|c| matches!(c, Component::Normal(_)))
            .collect::<PathBuf>();
        path.push(safe_rel);
        path
    }

    async fn serve_file(path: &PathBuf, method: &Method) -> Result<Resp, std::io::Error> {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let content_type = if mime.type_() == mime_guess::mime::TEXT {
            format!("{}; charset=utf-8", mime)
        } else {
            mime.to_string()
        };
        let metadata = tokio::fs::metadata(path).await?;
        let mut builder = Response::builder()
            .status(200)
            .header("Content-Type", content_type);
        if let Ok(time) = metadata.modified() {
            let datetime = httpdate::fmt_http_date(time);
            builder = builder.header("Last-Modified", datetime);
        }
        if method == Method::HEAD {
            return Ok(builder
                .body(miko_core::fast_builder::box_empty_body())
                .unwrap());
        }
        let file_len = metadata.len();
        let file = File::open(path).await?;
        let stream = ReaderStream::new(file);
        let body = FallibleStreamBody::with_size_hint(stream, file_len);
        Ok(builder.body(body.boxed()).unwrap())
    }
}

/// 静态文件服务构建器
pub struct StaticSvcBuilder {
    pub root: PathBuf,
    pub spa_fallback: bool,
    pub cors_layer: Option<CorsLayer>,
}
impl StaticSvcBuilder {
    /// 创建构建器
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            spa_fallback: false,
            cors_layer: None,
        }
    }
    /// 启用/关闭单页应用回退（当命中文件不存在时回退到 index.html）
    pub fn with_spa_fallback(mut self, spa_fallback: bool) -> Self {
        self.spa_fallback = spa_fallback;
        self
    }
    /// 配置 CORS Layer
    pub fn with_cors(mut self, cors_layer: CorsLayer) -> Self {
        self.cors_layer = Some(cors_layer);
        self
    }
    /// 允许任意跨域（开发便捷）
    pub fn cors_any(mut self) -> Self {
        self.cors_layer = Some(CorsLayer::permissive());
        self
    }
    /// 构建为可挂载的 Service
    pub fn build(self) -> HttpSvc<Req> {
        let service = StaticSvc {
            root: Arc::new(self.root),
            spa_fallback: self.spa_fallback,
        };
        if let Some(cors_layer) = self.cors_layer {
            BoxCloneService::new(cors_layer.clone().layer(service))
        } else {
            BoxCloneService::new(service)
        }
    }
}

impl Service<Req> for StaticSvc {
    type Response = Resp;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let root = self.root.clone();
        let spa_fallback = self.spa_fallback;
        let path = self.resolve_path(req.uri().path());
        Box::pin(async move {
            match StaticSvc::serve_file(&path, req.method()).await {
                Ok(resp) => Ok(resp),
                Err(e) => {
                    if spa_fallback && e.kind() == std::io::ErrorKind::NotFound {
                        let index_path = root.join("index.html");
                        match StaticSvc::serve_file(&index_path, req.method()).await {
                            Ok(resp) => Ok(resp),
                            Err(_) => Ok(crate::AppError::NotFound("File not found".to_string())
                                .into_response()),
                        }
                    } else {
                        Ok(crate::AppError::NotFound("File not found".to_string()).into_response())
                    }
                }
            }
        })
    }
}
