use crate::error::app_error::TRACE_ID;
use crate::handler::{Req, Resp};
use crate::router::Router;
use crate::{AppError, IntoResponse};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

pub struct RouterSvc<S> {
    pub router: Router<S>,
}
impl<S> Clone for RouterSvc<S> {
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
        }
    }
}

impl<S: Send + Sync + 'static> Service<Req> for RouterSvc<S> {
    type Response = Resp;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Resp, AppError>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Req) -> Self::Future {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let result = self.router.find_handler(&method, &path);

        // 自动设置 trace_id
        // 优先从请求头获取,如果没有则生成新的
        let trace_id = extract_or_generate_trace_id(&req);
        let trace_id_clone = trace_id.clone();

        let start = std::time::Instant::now();

        let task_future = async move {
            tracing::debug!(
                method = %method,
                path = %path,
                trace_id = %trace_id,
                "Request started"
            );
            let resp_result = match result {
                Some((mut handler, params)) => {
                    req.extensions_mut().insert(params);
                    handler.call(req).await
                }
                None => Ok(AppError::NotFound("404 Not Found".to_string()).into_response()),
            };
            // 记录请求完成
            let elapsed = start.elapsed();
            match &resp_result {
                Ok(response) => {
                    tracing::debug!(
                        method = %method,
                        path = %path,
                        trace_id = %trace_id,
                        status = %response.status(),
                        elapsed_ms = elapsed.as_millis(),
                        "Request completed"
                    );
                }
                Err(err) => {
                    tracing::debug!(
                        method = %method,
                        path = %path,
                        trace_id = %trace_id,
                        status = "500",
                        elapsed_ms = elapsed.as_millis(),
                        err = ?err,
                        "Request completed with error"
                    );
                }
            }
            Ok(resp_result.unwrap_or_else(|e| e.into_response()))
        };
        Box::pin(TRACE_ID.scope(trace_id_clone, task_future))
    }
}

/// 从请求中提取或生成 trace_id
///
/// 按优先级尝试:
/// 1. 从 `x-trace-id` 请求头获取
/// 2. 从 `x-request-id` 请求头获取
/// 3. 生成基于时间戳的 trace_id
fn extract_or_generate_trace_id(req: &Req) -> String {
    req.headers()
        .get("x-trace-id")
        .or_else(|| req.headers().get("x-request-id"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(generate_trace_id)
}

/// 生成 trace_id
///
/// 格式: `trace-{timestamp_micros}-{random}`
fn generate_trace_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros();

    // 使用线程ID和时间戳组合,避免冲突
    let thread_id = std::thread::current().id();
    format!("trace-{:x}-{:?}", timestamp, thread_id)
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}
