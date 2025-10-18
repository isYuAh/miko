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

pub struct SseEvent {
    pub data: String,
    pub event: Option<String>,
    pub id: Option<String>,
    pub retry: Option<u32>,
}

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

pub struct SseSendResult(Result<(), SendError<SseEvent>>);
impl SseSendResult {
    pub fn or_break(self) {
        if self.0.is_err() {
            panic_any(SseClientDisconnected);
        }
    }
}

impl SseEvent {
    pub fn data(dat: impl Into<String>) -> Self {
        Self {
            data: dat.into(),
            event: None,
            id: None,
            retry: None,
        }
    }
    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
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

pub struct SseSender {
    inner: Sender<SseEvent>,
}
impl SseSender {
    pub fn new(sender: Sender<SseEvent>) -> Self {
        Self { inner: sender }
    }
    pub async fn send(&self, data: impl IntoSseEvent) -> SseSendResult {
        SseSendResult(self.inner.send(data.into_sse_event()).await)
    }
    pub async fn send_event(&self, data: SseEvent) -> SseSendResult {
        SseSendResult(self.inner.send(data).await)
    }
    pub async fn event(&self, event: impl Into<String>, data: impl IntoSseEvent) -> SseSendResult {
        SseSendResult(
            self.inner
                .send(data.into_sse_event().event(event.into()))
                .await,
        )
    }
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
    pub fn inner(&self) -> &Sender<SseEvent> {
        &self.inner
    }
}
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

/// 设置一个全局的panic_hook，能够获取SseSender的send方法获取的结果调用or_break时
///
/// 如果是SseClientDisconnected的panic，则不会打印panic信息，直接结束任务
///
/// 其他panic则调用默认的panic_hook。
pub fn set_sse_panic_hook() {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        if sse_panic_hook_handler(panic_info) {
            return;
        }
        default_hook(panic_info);
    }));
}
pub fn sse_panic_hook_handler(panic_info: &PanicHookInfo) -> bool {
    panic_info.payload().is::<SseClientDisconnected>()
}
