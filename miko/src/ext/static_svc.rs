use std::convert::Infallible;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use http_body_util::{BodyExt};
use hyper::{Method, Response};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tower::{Layer, Service};
use tower::util::BoxCloneService;
use tower_http::cors::CorsLayer;
use miko_core::{decode_path, Req, Resp};
use miko_core::fallible_stream_body::FallibleStreamBody;
use miko_core::fast_builder::{ResponseBuilder};
use crate::handler::router::HttpSvc;

#[derive(Clone)]
pub struct StaticSvc {
    pub root: Arc<PathBuf>,
    pub spa_fallback: bool,
}
impl StaticSvc {
    pub fn builder(root: impl Into<PathBuf>) -> StaticSvcBuilder {
        StaticSvcBuilder::new(root)
    }
    fn resolve_path(&self, uri_path: &str) -> PathBuf {
        let mut path = self.root.as_ref().clone();
        let decoded = decode_path(uri_path);
        let safe_rel = Path::new(&decoded).components()
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
            return Ok(builder.body(miko_core::fast_builder::box_empty_body()).unwrap());
        }
        let file_len = metadata.len();
        let file = File::open(path).await?;
        let stream = ReaderStream::new(file);
        let body = FallibleStreamBody::with_size_hint(stream, file_len);
        Ok(builder.body(body.boxed()).unwrap())
    }
}

pub struct StaticSvcBuilder {
    pub root: PathBuf,
    pub spa_fallback: bool,
    pub cors_layer: Option<CorsLayer>
}
impl StaticSvcBuilder {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            spa_fallback: false,
            cors_layer: None
        }
    }
    pub fn with_spa_fallback(mut self, spa_fallback: bool) -> Self {
        self.spa_fallback = spa_fallback;
        self
    }
    pub fn with_cors(mut self, cors_layer: CorsLayer) -> Self {
        self.cors_layer = Some(cors_layer);
        self
    }
    pub fn cors_any(mut self) -> Self {
        self.cors_layer = Some(CorsLayer::permissive());
        self
    }
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
                            Err(_) => Ok(ResponseBuilder::not_found().unwrap()),
                        }
                    } else {
                        Ok(ResponseBuilder::not_found().unwrap())
                    }
                }
            }
        })
    }
}