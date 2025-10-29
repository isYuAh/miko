use crate::extractor::from_request::FromRequest;
use crate::handler::{FnOnceTuple, Req, TypedHandler, handler_to_svc};
use crate::http::response::into_response::IntoResponse;
use crate::router::HttpSvc;
use bytes::Bytes;
use http_body::Body;
use http_body_util::BodyExt;
use hyper::Response;
use std::convert::Infallible;
use std::future::Future;
use std::sync::Arc;
use tower::{Layer, Service, ServiceExt, util::BoxCloneService};

/// Layer 扩展 trait，为 handler 和 service 提供链式调用的 layer 功能
///
/// # 使用示例
///
/// ```rust,ignore
/// use miko::endpoint::LayerExt;
/// use tower_http::timeout::TimeoutLayer;
/// use std::time::Duration;
/// use miko::handler::{handler_to_svc};
/// use miko::router::HttpSvc;
/// use std::sync::Arc;
/// use tower_http::compression::CompressionLayer;
/// use miko::handler::TypedHandler;
///
/// async fn my_handler() -> String {
///     "Hello".to_string()
/// }
///
/// // 链式调用多个 layer，最终得到一个 Service
/// let endpoint = handler_to_svc(Arc::new(TypedHandler::new(my_handler, Arc::new(()))))
///     .layer(TimeoutLayer::new(Duration::from_secs(30)))
///     .layer(CompressionLayer::new());
///
/// // 使用 xxx_service 方法注册
/// router.get_service("/api/users", endpoint);
/// ```
pub trait LayerExt<Svc>: Sized {
    /// 为当前 handler 或 service 应用一个 layer，返回包装后的 Service
    fn layer<L, B>(self, layer: L) -> HttpSvc<Req>
    where
        Svc: Service<Req>,
        L: Layer<Svc>,
        L::Service:
            Service<Req, Response = Response<B>, Error = Infallible> + Clone + Send + 'static,
        B: Body<Data = Bytes, Error = Infallible> + Send + Sync + 'static,
        <L::Service as Service<Req>>::Future: Send + 'static;

    /// 为当前 handler 或 service 应用一个 layer（使用 with_layer 别名）
    fn with_layer<L, B>(self, layer: L) -> HttpSvc<Req>
    where
        Svc: Service<Req>,
        L: Layer<Svc>,
        L::Service:
            Service<Req, Response = Response<B>, Error = Infallible> + Clone + Send + 'static,
        B: Body<Data = Bytes, Error = Infallible> + Send + Sync + 'static,
        <L::Service as Service<Req>>::Future: Send + 'static,
    {
        self.layer(layer)
    }
}

/// 给handler应用state
pub trait WithState<S>: Sized {
    fn with_state<A, Fut, R, M>(self, state: S) -> HttpSvc<Req>
    where
        Self: FnOnceTuple<A, Output = Fut> + Clone + Send + Sync + 'static,
        A: FromRequest<S, M> + Send + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
        M: Send + Sync + 'static;
}

impl LayerExt<HttpSvc<Req>> for HttpSvc<Req> {
    fn layer<L, B>(self, layer: L) -> HttpSvc<Req>
    where
        L: Layer<Self>,
        L::Service:
            Service<Req, Response = Response<B>, Error = Infallible> + Clone + Send + 'static,
        B: Body<Data = Bytes, Error = Infallible> + Send + Sync + 'static,
        <L::Service as Service<Req>>::Future: Send + 'static,
    {
        let layered_svc = layer.layer(self).map_response(|resp| {
            let (parts, body) = resp.into_parts();
            let body = body.boxed();
            Response::from_parts(parts, body)
        });
        BoxCloneService::new(layered_svc)
    }
}

impl<F, S> WithState<S> for F
where
    S: Send + Sync + 'static,
{
    fn with_state<A, Fut, R, M>(self, state: S) -> HttpSvc<Req>
    where
        F: FnOnceTuple<A, Output = Fut> + Clone + Send + Sync + 'static,
        A: FromRequest<S, M> + Send + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
        M: Send + Sync + 'static,
    {
        let handler_arc = Arc::new(TypedHandler::new(self, Arc::new(state)));
        handler_to_svc(handler_arc)
    }
}
