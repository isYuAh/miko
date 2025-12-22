use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{HeaderMap, StatusCode};
use miko_core::Resp;
use serde::de::DeserializeOwned;

pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}
impl TestResponse {
    pub async fn from_response(resp: Resp) -> Self {
        let (parts, body) = resp.into_parts();
        let bytes = body
            .collect()
            .await
            .expect("Failed to collect body")
            .to_bytes();
        TestResponse {
            status: parts.status,
            headers: parts.headers,
            body: bytes,
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    pub fn bytes(&self) -> Bytes {
        self.body.clone()
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
    #[track_caller]
    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body).unwrap_or_else(|e| {
            panic!(
                "Failed to deserialize response\nerror={:?}\nbody={}",
                e,
                self.text()
            );
        })
    }

    #[track_caller]
    /// 断言响应状态为成功 (2xx)
    pub fn assert_success(&self) {
        assert!(
            self.status.is_success(),
            "Expected success status, got {}",
            self.status
        );
    }
    #[track_caller]
    pub fn assert_ok(&self) {
        self.assert_status(StatusCode::OK)
    }
    #[track_caller]
    /// 断言响应状态码
    pub fn assert_status(&self, expected: impl Into<StatusCode>) {
        let expected = expected.into();
        assert_eq!(
            self.status, expected,
            "Expected status {}, got {}",
            expected, self.status
        );
    }
    #[track_caller]
    pub fn assert_header<K, V>(&self, key: K, expected: V)
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let header_value = self.headers.get(key.as_ref()).unwrap_or_else(|| {
            panic!("Header {} not found in response", key.as_ref());
        });
        assert_eq!(
            header_value
                .to_str()
                .expect("Failed to convert header to str"),
            expected.as_ref(),
            "value of header {} does not match",
            key.as_ref()
        );
    }
    #[track_caller]
    pub fn assert_text(&self, expected: &str) {
        let text = self.text();
        assert_eq!(text, expected, "Response text does not match");
    }
    #[track_caller]
    pub fn assert_json<T: DeserializeOwned + PartialEq + std::fmt::Debug>(&self, expected: T) {
        let json: T = self.json();
        assert_eq!(json, expected, "Response JSON does not match");
    }
}
