use anyhow::anyhow;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use futures::stream::{SplitSink, SplitStream};
use hyper::upgrade::{Upgraded};
use hyper_util::rt::TokioIo;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio::task::JoinHandle;
use tokio_tungstenite::WebSocketStream;
use tungstenite::{Error, Message, Utf8Bytes};
use tungstenite::protocol::{Role, WebSocketConfig};
use crate::handler::extractor::extractors::Json;
use crate::handler::handler::{Req, Resp};
use crate::handler::ws::toolkit::upgrade_websocket;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum WsEvent {
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

pub struct WsSocket {
    io: WebSocketStream<TokioIo<Upgraded>>,
}
impl WsSocket {
    pub fn new(io: WebSocketStream<TokioIo<Upgraded>>) -> WsSocket {
        Self {
            io,
        }
    }
    pub async fn send(&mut self, msg: impl IntoMessage) -> tungstenite::Result<()> {
        self.io.send(msg.into_message()).await
    }
    pub async fn next(&mut self) -> Option<Result<Message, Error>> {
        self.io.next().await
    }
    pub async fn close(&mut self) -> tungstenite::Result<()> {
        self.io.close(None).await
    }
    pub fn split_inner(self) -> (WsSendSink, WsRecvStream) {
        self.io.split()
    }
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
                            eprintln!("websocket send error: {:?}", e);
                        }
                    }
                    break;
                }
            }
        });
        (WsSender::new(tx), WsReceiver::new(r), handle)
    }
}

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
        Message::Text(Utf8Bytes::from(serde_json::to_string(&self.0).unwrap_or_default()))
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



pub fn spawn_ws_event<F, Fut>(task: F, mut req: Req, options: Option<WebSocketConfig>) -> Result<Resp, anyhow::Error>
where
    F: FnOnce(WsSocket) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let Ok((resp, upgrade)) = upgrade_websocket(&mut req) else {
        return Err(anyhow!("failed to upgrade websocket"));
    };
    tokio::spawn(async move {
        let upgraded = upgrade.await;
        match upgraded {
            Ok(upgraded) => {
                let io = WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, options).await;
                task(WsSocket::new(io)).await;
            }
            Err(_e) => {
                panic!("failed to upgrade websocket");
            }
        }
    });
    Ok(resp)
}
pub type WsSendSink = SplitSink<WebSocketStream<TokioIo<Upgraded>>, Message>;
pub type WsRecvStream = SplitStream<WebSocketStream<TokioIo<Upgraded>>>;
#[derive(Clone)]
pub struct WsSender {
    inner: mpsc::Sender<Message>
}
impl WsSender {
    pub fn new(inner: mpsc::Sender<Message>) -> Self {
        Self { inner }
    }
}
pub struct WsReceiver {
    inner: WsRecvStream
}
impl WsReceiver {
    pub fn new(inner: WsRecvStream) -> Self {
        Self { inner  }
    }
}
impl WsSender {
    pub async fn send(&mut self, msg: impl IntoMessage) -> Result<(), SendError<Message>> {
        self.inner.send(msg.into_message()).await
    }
}
impl WsReceiver {
    pub async fn next(&mut self) -> Option<Result<Message, Error>> {
        self.inner.next().await
    }
}