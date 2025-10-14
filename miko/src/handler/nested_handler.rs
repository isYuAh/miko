use crate::handler::extractor::path_params::PathParams;
use crate::handler::handler::{Req, Resp};
use hyper::http::uri::PathAndQuery;
use hyper::{Request, Uri};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct NestLayer {
    prefix: Arc<String>,
}
impl NestLayer {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: Arc::new(prefix.into()),
        }
    }
}
#[derive(Clone)]
pub struct NestSvc<S> {
    inner: S,
    prefix: Arc<String>,
}

impl<S> Layer<S> for NestLayer {
    type Service = NestSvc<S>;
    fn layer(&self, inner: S) -> Self::Service {
        NestSvc {
            inner,
            prefix: self.prefix.clone(),
        }
    }
}

impl<S> Service<Req> for NestSvc<S>
where
    S: Service<Req, Response = Resp, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Resp;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Resp, Infallible>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let mut inner = self.inner.clone();
        let prefix = self.prefix.clone();

        Box::pin(async move {
            let (mut parts, body) = req.into_parts();

            // 偏移 PathParams
            let (new_uri, pcount) = strip_prefix_preserve_query(&parts.uri, &prefix);
            if pcount > 0 {
                if let Some(pp) = parts.extensions.remove::<PathParams>() {
                    parts.extensions.insert(pp.shift_count(pcount));
                }
            }
            parts.uri = new_uri;

            let req2 = Request::from_parts(parts, body);
            inner.call(req2).await
        })
    }
}

fn count_leading_params(prefix: &str) -> (usize, usize) {
    let mut segs_to_drop = 0usize;
    let mut params_in_prefix = 0usize;
    for seg in prefix.trim_matches('/').split('/') {
        if seg.is_empty() {
            continue;
        }
        segs_to_drop += 1;
        let b = seg.as_bytes();
        if b.first() == Some(&b'{') && b.last() == Some(&b'}') {
            params_in_prefix += 1;
        }
    }
    (segs_to_drop, params_in_prefix)
}

fn strip_prefix_preserve_query(uri: &Uri, prefix: &str) -> (Uri, usize) {
    let (segs_to_drop, params_in_prefix) = count_leading_params(prefix);

    let path = uri.path();
    let bytes = path.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i] == b'/' {
        i += 1;
    }

    let mut cut = 0usize;
    if segs_to_drop > 0 {
        let mut dropped = 0usize;
        loop {
            while i < bytes.len() && bytes[i] != b'/' {
                i += 1;
            }
            let next_slash = i.min(bytes.len());
            while i < bytes.len() && bytes[i] == b'/' {
                i += 1;
            }
            dropped += 1;
            cut = next_slash;
            if dropped >= segs_to_drop || i >= bytes.len() {
                break;
            }
        }
    }

    let new_path = if cut >= path.len() {
        "/".to_string()
    } else {
        path[cut..].to_string()
    };
    let new_paq = match uri.query() {
        Some(q) if !q.is_empty() => format!("{new_path}?{q}"),
        _ => new_path,
    };

    let mut parts = uri.clone().into_parts();
    parts.path_and_query =
        Some(PathAndQuery::from_str(&new_paq).unwrap_or_else(|_| PathAndQuery::from_static("/")));
    let new_uri = Uri::from_parts(parts).unwrap_or_else(|_| Uri::from_static("/"));
    (new_uri, params_in_prefix)
}
