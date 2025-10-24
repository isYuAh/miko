use crate::http::response::into_response::IntoResponse;
use crate::router::HttpSvc;
use http_body_util::BodyExt;
use hyper::{Method, Response, StatusCode, header};
use miko_core::fallible_stream_body::FallibleStreamBody;
use miko_core::{Req, Resp, decode_path};
use std::convert::Infallible;
use std::future::Future;
use std::io::SeekFrom;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
use tower::util::BoxCloneService;
use tower::{Layer, Service};
use tower_http::cors::CorsLayer;

/// 静态文件服务，实现目录下文件的按路径映射与可选 SPA 回退
#[derive(Clone)]
pub struct StaticSvc {
    pub root: Arc<PathBuf>,
    pub spa_fallback: bool,
    pub fallback_files: Arc<Vec<String>>,
    pub index_files: Arc<Vec<String>>,
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

    async fn resolve_index_file(&self, path: PathBuf) -> Option<PathBuf> {
        if let Ok(metadata) = tokio::fs::metadata(&path).await {
            if metadata.is_dir() {
                for index_file in self.index_files.iter() {
                    let index_path = path.join(index_file);
                    if tokio::fs::metadata(&index_path).await.is_ok() {
                        return Some(index_path);
                    }
                }
            }
        }
        None
    }

    async fn try_fallback_files(&self, root: &PathBuf) -> Option<PathBuf> {
        for fallback_file in self.fallback_files.iter() {
            let fallback_path = root.join(fallback_file);
            if tokio::fs::metadata(&fallback_path).await.is_ok() {
                return Some(fallback_path);
            }
        }
        None
    }

    fn parse_range(range_header: &str, file_size: u64) -> Option<(u64, u64)> {
        if !range_header.starts_with("bytes=") {
            return None;
        }
        let range_str = &range_header[6..];
        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() != 2 {
            return None;
        }

        let start = parts[0].parse::<u64>().ok()?;
        let end = if parts[1].is_empty() {
            file_size - 1
        } else {
            parts[1].parse::<u64>().ok()?.min(file_size - 1)
        };

        if start <= end && start < file_size {
            Some((start, end))
        } else {
            None
        }
    }

    async fn serve_file(path: &PathBuf, method: &Method, req: &Req) -> Result<Resp, std::io::Error> {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let content_type = if mime.type_() == mime_guess::mime::TEXT {
            format!("{}; charset=utf-8", mime)
        } else {
            mime.to_string()
        };
        let metadata = tokio::fs::metadata(path).await?;
        let file_size = metadata.len();

        let etag = if let Ok(modified) = metadata.modified() {
            format!(
                "\"{:x}-{:x}\"",
                modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                file_size
            )
        } else {
            format!("\"{:x}\"", file_size)
        };

        if let Some(if_none_match) = req.headers().get(header::IF_NONE_MATCH) {
            if let Ok(if_none_match_str) = if_none_match.to_str() {
                if if_none_match_str == etag || if_none_match_str == "*" {
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_MODIFIED)
                        .header(header::ETAG, etag)
                        .body(miko_core::fast_builder::box_empty_body())
                        .unwrap());
                }
            }
        }

        let mut builder = Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .header(header::ETAG, etag)
            .header(header::ACCEPT_RANGES, "bytes");

        if let Ok(time) = metadata.modified() {
            let datetime = httpdate::fmt_http_date(time);
            builder = builder.header(header::LAST_MODIFIED, datetime);
        }

        // 处理 Range
        if let Some(range_header) = req.headers().get(header::RANGE) {
            if let Ok(range_str) = range_header.to_str() {
                if let Some((start, end)) = Self::parse_range(range_str, file_size) {
                    let content_length = end - start + 1;
                    let mut file = File::open(path).await?;
                    file.seek(SeekFrom::Start(start)).await?;
                    
                    builder = builder
                        .status(StatusCode::PARTIAL_CONTENT)
                        .header(header::CONTENT_LENGTH, content_length)
                        .header(
                            header::CONTENT_RANGE,
                            format!("bytes {}-{}/{}", start, end, file_size),
                        );

                    if method == Method::HEAD {
                        return Ok(builder
                            .body(miko_core::fast_builder::box_empty_body())
                            .unwrap());
                    }

                    let limited_file = file.take(content_length);
                    let stream = ReaderStream::new(limited_file);
                    let body = FallibleStreamBody::with_size_hint(stream, content_length);
                    return Ok(builder.body(body.boxed()).unwrap());
                }
            }
        }

        builder = builder
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, file_size);

        if method == Method::HEAD {
            return Ok(builder
                .body(miko_core::fast_builder::box_empty_body())
                .unwrap());
        }

        let file = File::open(path).await?;
        let stream = ReaderStream::new(file);
        let body = FallibleStreamBody::with_size_hint(stream, file_size);
        Ok(builder.body(body.boxed()).unwrap())
    }
}

/// 静态文件服务构建器
pub struct StaticSvcBuilder {
    pub root: PathBuf,
    pub spa_fallback: bool,
    pub fallback_files: Vec<String>,
    pub index_files: Vec<String>,
    pub cors_layer: Option<CorsLayer>,
}
impl StaticSvcBuilder {
    /// 创建构建器
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            spa_fallback: false,
            fallback_files: vec!["index.html".to_string()],
            index_files: vec!["index.html".to_string(), "index.htm".to_string()],
            cors_layer: None,
        }
    }
    /// 启用/关闭单页应用回退（当命中文件不存在时回退到配置的 fallback 文件）
    pub fn with_spa_fallback(mut self, spa_fallback: bool) -> Self {
        self.spa_fallback = spa_fallback;
        self
    }
    /// 自定义 SPA 回退文件列表（按顺序尝试）
    /// 
    /// # 示例
    /// ```no_run
    /// # use miko::ext::StaticSvc;
    /// StaticSvc::builder("./dist")
    ///     .with_spa_fallback(true)
    ///     .with_fallback_files(vec!["index.html", "app.html", "404.html"])
    ///     .build();
    /// ```
    pub fn with_fallback_files(mut self, files: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.fallback_files = files.into_iter().map(|f| f.into()).collect();
        self
    }
    /// 配置目录索引文件列表（当访问目录时按顺序尝试）
    /// 
    /// # 示例
    /// ```no_run
    /// # use miko::ext::StaticSvc;
    /// StaticSvc::builder("./public")
    ///     .with_index_files(vec!["index.html", "index.htm", "default.html"])
    ///     .build();
    /// ```
    pub fn with_index_files(mut self, files: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.index_files = files.into_iter().map(|f| f.into()).collect();
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
            fallback_files: Arc::new(self.fallback_files),
            index_files: Arc::new(self.index_files),
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
        let mut path = self.resolve_path(req.uri().path());
        
        let self_clone = self.clone();
        Box::pin(async move {
            if let Some(index_path) = self_clone.resolve_index_file(path.clone()).await {
                path = index_path;
            }

            match StaticSvc::serve_file(&path, req.method(), &req).await {
                Ok(resp) => Ok(resp),
                Err(e) => {
                    if spa_fallback && e.kind() == std::io::ErrorKind::NotFound {
                        if let Some(fallback_path) = self_clone.try_fallback_files(&root).await {
                            match StaticSvc::serve_file(&fallback_path, req.method(), &req).await {
                                Ok(resp) => return Ok(resp),
                                Err(_) => {}
                            }
                        }
                    }
                    Ok(crate::AppError::NotFound("File not found".to_string()).into_response())
                }
            }
        })
    }
}
