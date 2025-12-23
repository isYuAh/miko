use crate::miko_core::{Req, Resp};
use crate::{AppError, AppResult};
use std::future::Future;
use std::task::{Context, Poll};
use tower::util::BoxCloneService;
use tower::{Layer, Service};

pub struct Next {
    inner: BoxCloneService<Req, Resp, AppError>,
}

impl Next {
    /// Execute the next middleware or handler
    pub async fn run(mut self, req: Req) -> AppResult<Resp> {
        self.inner.call(req).await
    }
}

/// Use async functions as middleware
///
/// # Example
///
/// ```rust,ignore
/// use miko::middleware::{middleware_from_fn, Next};
/// use miko::http::{Req, Resp};
/// use miko::AppResult;
///
/// async fn my_middleware(req: Req, next: Next) -> AppResult<Resp> {
///     println!("Request: {:?}", req);
///     let resp = next.run(req).await?;
///     println!("Response: {:?}", resp);
///     Ok(resp)
/// }
///
/// // app.with_layer(middleware_from_fn(my_middleware));
/// ```
pub fn middleware_from_fn<F>(f: F) -> FromFnLayer<F> {
    FromFnLayer { f }
}

/// Layer created by `from_fn`
#[derive(Clone, Copy)]
pub struct FromFnLayer<F> {
    f: F,
}

impl<S, F> Layer<S> for FromFnLayer<F>
where
    F: Clone,
    S: Service<Req, Response = Resp, Error = AppError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = MiddlewareFromFn<F>;

    fn layer(&self, inner: S) -> Self::Service {
        MiddlewareFromFn {
            f: self.f.clone(),
            inner: BoxCloneService::new(inner),
        }
    }
}

/// Service created by `from_fn`
#[derive(Clone)]
pub struct MiddlewareFromFn<F> {
    f: F,
    inner: BoxCloneService<Req, Resp, AppError>,
}

impl<F, Fut> Service<Req> for MiddlewareFromFn<F>
where
    F: Fn(Req, Next) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = AppResult<Resp>> + Send + 'static,
{
    type Response = Resp;
    type Error = AppError;
    type Future = Fut;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let next = Next {
            inner: self.inner.clone(),
        };
        (self.f)(req, next)
    }
}
