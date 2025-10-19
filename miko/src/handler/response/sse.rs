use crate::handler::extractor::extractors::Json;
use crate::handler::handler::Resp;
use crate::handler::into_response::{IntoResponse, SSE};
use bytes::Bytes;
use futures::StreamExt;
use serde::Serialize;
use std::convert::Infallible;
use std::panic;
use std::panic::{PanicHookInfo, panic_any};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Sender, channel};
use tokio_stream::wrappers::ReceiverStream;

/// 一个 Server-Sent Event 事件对象
///
/// 使用 SseSender::send(…)/event(…) 时可直接传入 &str/String/Json<T>，也可手动构建 SseEvent。
pub struct SseEvent {
    pub data: String,
    pub event: Option<String>,
    pub id: Option<String>,
    pub retry: Option<u32>,
}

/// 当客户端断开连接时，通过 panic 控制流快速结束任务的标记类型
pub struct SseClientDisconnected;

impl std::fmt::Display for SseClientDisconnected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SSE client disconnected, task terminating.")
    }
}

impl std::fmt::Debug for SseClientDisconnected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SseClientDisconnected")
    }
}

/// 发送结果包装，提供 or_break() 在断连时终止任务
pub struct SseSendResult(Result<(), SendError<SseEvent>>);
impl SseSendResult {
    /// 若客户端已断连，调用该方法将触发一个可被自定义 panic_hook 吞掉的 panic
    pub fn or_break(self) {
        if self.0.is_err() {
            panic_any(SseClientDisconnected);
        }
    }
}

impl SseEvent {
    /// 构建 data 字段
    pub fn data(dat: impl Into<String>) -> Self {
        Self {
            data: dat.into(),
            event: None,
            id: None,
            retry: None,
        }
    }
    /// 设置 event 名称
    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }
    /// 设置 id
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    /// 设置重试时间（毫秒）
    pub fn retry(mut self, retry: u32) -> Self {
        self.retry = Some(retry);
        self
    }
    pub(crate) fn to_bytes(&self) -> Bytes {
        let mut buf = String::new();
        if let Some(event) = &self.event {
            buf.push_str(&format!("event: {}\n", event));
        }
        if let Some(id) = &self.id {
            buf.push_str(&format!("id: {}\n", id));
        }
        for line in self.data.lines() {
            buf.push_str(&format!("data: {}\n", line));
        }
        if let Some(retry) = self.retry {
            buf.push_str(&format!("retry: {}\n", retry));
        }
        buf.push_str("\n");
        Bytes::from(buf)
    }
}
impl From<String> for SseEvent {
    fn from(data: String) -> Self {
        SseEvent::data(data)
    }
}
impl From<&str> for SseEvent {
    fn from(data: &str) -> Self {
        SseEvent::data(data)
    }
}
impl<T: Serialize> From<Json<T>> for SseEvent {
    fn from(value: Json<T>) -> Self {
        SseEvent::data(serde_json::to_string(&value.0).unwrap_or_default())
    }
}

/// 启动一个 SSE 任务并返回响应
///
/// 参数为一个闭包，框架会创建 SseSender 并在后台任务中运行你的逻辑。
pub fn spawn_sse_event<F, Fut>(task: F) -> impl IntoResponse
where
    F: FnOnce(SseSender) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (tx, rx) = channel::<SseEvent>(32);
    tokio::spawn(task(SseSender::new(tx)));
    let stream = ReceiverStream::new(rx).map(|event| Ok::<Bytes, Infallible>(event.to_bytes()));
    SSE(stream)
}

impl<F, Fut> IntoResponse for F
where
    F: FnOnce(SseSender) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fn into_response(self) -> Resp {
        spawn_sse_event(self).into_response()
    }
}

/// SSE 发送端，内部基于 mpsc::Sender
pub struct SseSender {
    inner: Sender<SseEvent>,
}
impl SseSender {
    /// 创建一个新的 SseSender
    pub fn new(sender: Sender<SseEvent>) -> Self {
        Self { inner: sender }
    }
    /// 发送数据（&str/String/Json<T>/SseEvent 均可），返回结果可调用 or_break()
    pub async fn send(&self, data: impl IntoSseEvent) -> SseSendResult {
        SseSendResult(self.inner.send(data.into_sse_event()).await)
    }
    /// 发送一个已构建的事件
    pub async fn send_event(&self, data: SseEvent) -> SseSendResult {
        SseSendResult(self.inner.send(data).await)
    }
    /// 发送指定 event 名称的事件
    pub async fn event(&self, event: impl Into<String>, data: impl IntoSseEvent) -> SseSendResult {
        SseSendResult(
            self.inner
                .send(data.into_sse_event().event(event.into()))
                .await,
        )
    }
    /// 渠道是否已关闭（客户端断开）
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
    /// 访问内部发送端
    pub fn inner(&self) -> &Sender<SseEvent> {
        &self.inner
    }
}

/// 可转换为 SseEvent 的类型
pub trait IntoSseEvent {
    fn into_sse_event(self) -> SseEvent;
}
impl IntoSseEvent for SseEvent {
    fn into_sse_event(self) -> SseEvent {
        self
    }
}
impl IntoSseEvent for String {
    fn into_sse_event(self) -> SseEvent {
        SseEvent::data(self)
    }
}
impl IntoSseEvent for &str {
    fn into_sse_event(self) -> SseEvent {
        SseEvent::data(self)
    }
}
impl<T: Serialize> IntoSseEvent for Json<T> {
    fn into_sse_event(self) -> SseEvent {
        SseEvent::data(serde_json::to_string(&self.0).unwrap_or_default())
    }
}

/// 设置一个全局 panic_hook，使 SseSender::send().or_break() 在断连时静默终止任务
///
/// 若 panic 为 SseClientDisconnected，将不会打印 panic 信息，其余 panic 委托给默认 hook。
pub fn set_sse_panic_hook() {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        if sse_panic_hook_handler(panic_info) {
            return;
        }
        default_hook(panic_info);
    }));
}
/// SSE panic_hook 的判定逻辑，返回 true 表示已处理（吞掉）
pub fn sse_panic_hook_handler(panic_info: &PanicHookInfo) -> bool {
    panic_info.payload().is::<SseClientDisconnected>()
}
