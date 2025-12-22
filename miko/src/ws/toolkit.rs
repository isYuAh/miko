use http_body_util::{BodyExt, Empty};
use hyper::{Response, StatusCode, header::CONNECTION, upgrade::OnUpgrade};
use hyper_util::rt::TokioIo;
use miko_core::{Req, Resp};
use tokio_tungstenite::WebSocketStream;
use tungstenite::error::ProtocolError;

/// 升级当前 HTTP 请求为 WebSocket，返回 101 响应和 OnUpgrade 句柄
pub type WsStream = WebSocketStream<TokioIo<hyper::upgrade::Upgraded>>;

/// 执行协议握手并返回 101 Switching Protocols 响应与升级句柄
pub fn upgrade_websocket(req: &mut Req) -> Result<(Resp, OnUpgrade), anyhow::Error> {
    let key = req
        .headers()
        .get(hyper::header::SEC_WEBSOCKET_KEY)
        .ok_or(ProtocolError::MissingSecWebSocketKey)?;
    if req
        .headers()
        .get(hyper::header::SEC_WEBSOCKET_VERSION)
        .map(|v| v.as_bytes())
        != Some(b"13")
    {
        return Err(ProtocolError::MissingSecWebSocketVersionHeader.into());
    }
    let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
    let resp = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(CONNECTION, "Upgrade")
        .header(hyper::header::UPGRADE, "websocket")
        .header(hyper::header::SEC_WEBSOCKET_ACCEPT, accept)
        .body(Empty::new().map_err(Into::into).boxed())
        .expect("failed to build response");
    let on_upgrade = hyper::upgrade::on(req);
    Ok((resp, on_upgrade))
}

/// 判断请求是否为 WebSocket 升级请求
pub fn is_upgrade_request<B>(request: &hyper::Request<B>) -> bool {
    header_contains_value(request.headers(), CONNECTION, "Upgrade")
        && header_contains_value(request.headers(), hyper::header::UPGRADE, "websocket")
}

fn header_contains_value(
    headers: &hyper::HeaderMap,
    header: impl hyper::header::AsHeaderName,
    value: impl AsRef<[u8]>,
) -> bool {
    let value = value.as_ref();
    for header in headers.get_all(header) {
        if header
            .as_bytes()
            .split(|&c| c == b',')
            .any(|x| trim(x).eq_ignore_ascii_case(value))
        {
            return true;
        }
    }
    false
}

fn trim(data: &[u8]) -> &[u8] {
    trim_end(trim_start(data))
}

fn trim_start(data: &[u8]) -> &[u8] {
    if let Some(start) = data.iter().position(|x| !x.is_ascii_whitespace()) {
        &data[start..]
    } else {
        b""
    }
}

fn trim_end(data: &[u8]) -> &[u8] {
    if let Some(last) = data.iter().rposition(|x| !x.is_ascii_whitespace()) {
        &data[..last + 1]
    } else {
        b""
    }
}
