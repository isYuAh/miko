use crate::extractor::Json;
use crate::handler::{Req, Resp};
use crate::ws::toolkit::upgrade_websocket;
use anyhow::anyhow;
use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio::task::JoinHandle;
use tokio_tungstenite::WebSocketStream;
use tungstenite::protocol::{Role, WebSocketConfig};
use tungstenite::{Error, Message, Utf8Bytes};

/// WebSocket 连接封装，提供便捷的发送/接收/split
pub struct WsSocket {
    io: WebSocketStream<TokioIo<Upgraded>>,
}
impl WsSocket {
    /// 基于底层流创建
    pub fn new(io: WebSocketStream<TokioIo<Upgraded>>) -> WsSocket {
        Self { io }
    }
    /// 发送一条消息
    pub async fn send(&mut self, msg: impl IntoMessage) -> tungstenite::Result<()> {
        self.io.send(msg.into_message()).await
    }
    /// 接收下一条消息
    pub async fn next(&mut self) -> Option<Result<Message, Error>> {
        self.io.next().await
    }
    /// 主动关闭连接
    pub async fn close(&mut self) -> tungstenite::Result<()> {
        self.io.close(None).await
    }
    /// 分离底层读写端
    pub fn split_inner(self) -> (WsSendSink, WsRecvStream) {
        self.io.split()
    }
    /// 分离为发送端与接收端（发送端通过 mpsc 发送，避免并发 Borrow 问题）
    pub fn split(self) -> (WsSender, WsReceiver, JoinHandle<()>) {
        let (mut w, r) = self.io.split();
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        let handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = w.send(msg).await {
                    match e {
                        Error::ConnectionClosed | Error::Protocol(_) => {
                            break;
                        }
                        _ => {
                            tracing::warn!(error = ?e, "WebSocket send error");
                        }
                    }
                    break;
                }
            }
        });
        (WsSender::new(tx), WsReceiver::new(r), handle)
    }
}

/// 可转换为 WebSocket Message 的类型
pub trait IntoMessage {
    fn into_message(self) -> Message;
}
impl IntoMessage for Message {
    fn into_message(self) -> Message {
        self
    }
}
impl IntoMessage for &str {
    fn into_message(self) -> Message {
        Message::Text(Utf8Bytes::from(self.to_string()))
    }
}
impl IntoMessage for String {
    fn into_message(self) -> Message {
        Message::Text(Utf8Bytes::from(self))
    }
}
impl<T: Serialize> IntoMessage for Json<T> {
    fn into_message(self) -> Message {
        Message::Text(Utf8Bytes::from(
            serde_json::to_string(&self.0).unwrap_or_default(),
        ))
    }
}
impl IntoMessage for Bytes {
    fn into_message(self) -> Message {
        Message::Binary(self)
    }
}
impl IntoMessage for Vec<u8> {
    fn into_message(self) -> Message {
        Message::Binary(self.into())
    }
}
impl IntoMessage for &[u8] {
    fn into_message(self) -> Message {
        Message::Binary(self.to_vec().into())
    }
}

/// 将当前请求升级为 WebSocket 并在后台运行你的异步任务
pub fn spawn_ws_event<F, Fut>(
    task: F,
    req: &mut Req,
    options: Option<WebSocketConfig>,
) -> Result<Resp, anyhow::Error>
where
    F: FnOnce(WsSocket) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let Ok((resp, upgrade)) = upgrade_websocket(req) else {
        return Err(anyhow!("failed to upgrade websocket"));
    };
    tokio::spawn(async move {
        let upgraded = upgrade.await;
        match upgraded {
            Ok(upgraded) => {
                let io =
                    WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, options)
                        .await;
                task(WsSocket::new(io)).await;
            }
            Err(_e) => {
                panic!("failed to upgrade websocket");
            }
        }
    });
    Ok(resp)
}

/// 底层 Split 类型别名
pub type WsSendSink = SplitSink<WebSocketStream<TokioIo<Upgraded>>, Message>;
/// 底层 Split 类型别名
pub type WsRecvStream = SplitStream<WebSocketStream<TokioIo<Upgraded>>>;

/// WebSocket 发送端（基于 mpsc 管道，便于跨任务发送）
#[derive(Clone)]
pub struct WsSender {
    inner: mpsc::Sender<Message>,
}
impl WsSender {
    pub fn new(inner: mpsc::Sender<Message>) -> Self {
        Self { inner }
    }
}

/// WebSocket 接收端（包装 SplitStream）
pub struct WsReceiver {
    inner: WsRecvStream,
}
impl WsReceiver {
    pub fn new(inner: WsRecvStream) -> Self {
        Self { inner }
    }
}
impl WsSender {
    /// 发送一条消息
    pub async fn send(&mut self, msg: impl IntoMessage) -> Result<(), SendError<Message>> {
        self.inner.send(msg.into_message()).await
    }
}
impl WsReceiver {
    /// 接收下一条消息
    pub async fn next(&mut self) -> Option<Result<Message, Error>> {
        self.inner.next().await
    }
}
