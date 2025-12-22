use crate::router::HttpSvc;
use crate::test::test_response::TestResponse;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Method;
use hyper::http::{HeaderName, HeaderValue, request};
use miko_core::Req;
use tower::ServiceExt;

pub struct TestClient {
    svc: HttpSvc<Req>,
}

macro_rules! define_mock_method {
    (
        $(
            $name:ident => $method:ident
        ),* $(,)?
    ) => {
        $(
            pub fn $name(&self, uri: &str) -> TestRequestBuilder {
                self.build(Method::$method, uri)
            }
        )*
    };
}

impl TestClient {
    pub fn new(svc: HttpSvc<Req>) -> Self {
        Self { svc }
    }
    fn build(&self, method: Method, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder {
            svc: self.svc.clone(),
            builder: request::Builder::new().method(method).uri(uri),
            body: Vec::new(),
        }
    }
    define_mock_method! {
        get => GET,
        post => POST,
        put => PUT,
        delete => DELETE,
        patch => PATCH,
        head => HEAD,
        options => OPTIONS,
        trace => TRACE,
        connect => CONNECT
    }
}

pub struct TestRequestBuilder {
    svc: HttpSvc<Req>,
    builder: request::Builder,
    body: Vec<u8>,
}

impl TestRequestBuilder {
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<HeaderName>,
        V: Into<HeaderValue>,
    {
        self.builder = self.builder.header(key.into(), value.into());
        self
    }

    pub fn json<T: serde::Serialize>(mut self, json: &T) -> Self {
        self.body = serde_json::to_vec(json).expect("Failed to serialize JSON");
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.body = text.as_bytes().to_vec();
        self
    }

    pub async fn send(self) -> TestResponse {
        let body = Full::new(Bytes::from(self.body))
            .map_err(Into::into)
            .boxed_unsync();

        let req = self.builder.body(body).expect("Failed to build request");
        let resp = self
            .svc
            .oneshot(req)
            .await
            .expect("Failed to execute request");
        TestResponse::from_response(resp).await
    }
}
