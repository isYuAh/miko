use hyper::{HeaderMap, StatusCode};
use miko::endpoint::LayerExt;
use miko::endpoint::layer::WithState;
use miko::ext::uploader::{DiskStorage, DiskStorageConfig, Uploader};
use miko::extractor::multipart::MultipartResult;
use miko::extractor::{Json, Query};
use miko::http::response::into_response::IntoResponse;
use miko::http::response::sse::SseSender;
use miko::macros::*;
use miko::ws::server::{IntoMessage, spawn_ws_event};
use miko::*;
use miko_core::Req;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct Item {
    id: u32,
    name: String,
    timestamp: u64,
}

#[route("/ws", method = "get")]
/// 你好啊
async fn ws_handler(mut req: Req) {
    spawn_ws_event(
        |mut io| async move {
            io.send("hello world").await.expect("websocket send error");
            let (mut w, mut r, _) = io.split();
            {
                let mut w = w.clone();
                tokio::spawn(async move {
                    w.send("START --".into_message())
                        .await
                        .expect("websocket send error");
                    loop {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        let msg = format!("server time: {}", now);
                        let _ = w.send(msg.into_message()).await;
                    }
                });
            }
            tokio::spawn(async move {
                while let Some(msg) = r.next().await {
                    let msg = msg.expect("websocket recv error");
                    if msg.is_text() {
                        let txt = msg.into_text().expect("websocket into text error");
                        let _ = w.send(txt.into_message()).await;
                        println!("recv text: {}", txt);
                    } else if msg.is_binary() {
                        let bin = msg.into_data();
                        println!("recv binary: {:?}", bin);
                    } else if msg.is_close() {
                        println!("websocket closed");
                        break;
                    }
                }
            });
        },
        &mut req,
        None,
    )
    .expect("failed to spawn websocket handler")
}

struct Service {}
#[component]
impl Service {
    async fn new() -> Self {
        Self {}
    }
    pub fn operation(&self) -> String {
        println!("operation");
        format!("{:?}", SystemTime::now())
    }
}

#[get("/sse")]
async fn sse_handler(#[config("value.haha.heihei")] v: String, #[dep] service: Arc<Service>) {
    |sender: SseSender| async move {
        sender.send(format!("{:?}", v)).await.or_break();
        tokio::time::sleep(Duration::from_secs(3)).await;
        sender.send(service.operation()).await.or_break();
        tokio::time::sleep(Duration::from_secs(5)).await;
        sender
            .send(Json(Item {
                id: 888,
                name: "sse item".into(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            }))
            .await
            .or_break();
        tokio::time::sleep(Duration::from_secs(5)).await;
        sender.send("endded").await.or_break();
    }
}

#[get("/test")]
async fn handler(
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if headers.get("X-Auth").and_then(|v| v.to_str().ok()) != Some("secret") {
        return (
            StatusCode::UNAUTHORIZED,
            Json(vec![Item {
                id: 0,
                name: "unauthorized".into(),
                timestamp: 0,
            }]),
        );
    }
    let uid: u32 = params.get("uid").and_then(|v| v.parse().ok()).unwrap_or(0);
    let name: String = params
        .get("name")
        .cloned()
        .unwrap_or_else(|| "guest".into());
    let mut count: u32 = params
        .get("count")
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    if count > 1000 {
        count = 1000;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let items: Vec<Item> = (0..count)
        .map(|i| Item {
            id: uid + i,
            name: name.clone(),
            timestamp: now,
        })
        .collect();

    (StatusCode::OK, Json(items))
}

#[get("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功")]
#[u_response(status = 404, description = "用户不存在")]
async fn get_user(
    #[path]
    #[desc("用户ID")]
    id: String,
) {
    println!("Get user: {}", "123");
}

#[miko(sse)]
async fn main() {
    tracing_subscriber::fmt::init();
    router.service(
        "/uploader",
        Uploader::single(DiskStorage::new(
            "uploads",
            DiskStorageConfig::default().max_size(50 * 1024),
        )),
    );
    router.get_service(
        "/111",
        (async move || "111").with_state(router.state.clone()),
    );
    router.cors_any();
    router.post("/ru", async move |mtp: MultipartResult| {
        let first = mtp.files.get("file");
        match first {
            Some(file) => {
                let file = file.first();
                if file.is_none() {
                    return (StatusCode::INTERNAL_SERVER_ERROR, "No File");
                }
                let file = file.unwrap();
                println!(
                    "file path: {:?}, size: {}",
                    file.linker.file_path, file.size
                );
                (StatusCode::OK, "OK OK get")
            }
            None => (StatusCode::INTERNAL_SERVER_ERROR, "No File"),
        }
    });
}
