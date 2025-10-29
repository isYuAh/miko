# WebSocket 支持

Miko 提供 WebSocket 支持，可以轻松实现双向实时通信。

## 基础用法

### 创建 WebSocket 路由

```rust
use miko::*;
use miko::macros::*;
use miko::ws::server::{spawn_ws_event, IntoMessage};
use std::time::Duration;

#[get("/ws")]
async fn websocket_handler(mut req: Req) {
    spawn_ws_event(
        |mut io| async move {
            // 发送欢迎消息
            io.send("Connected!").await.expect("send failed");

            // 循环接收和发送消息
            while let Some(msg) = io.recv().await {
                match msg {
                    Ok(message) if message.is_text() => {
                        let text = message.into_text().unwrap();
                        println!("Received: {}", text);

                        // 回显消息
                        io.send(format!("Echo: {}", text))
                            .await
                            .expect("send failed");
                    }
                    Ok(message) if message.is_close() => {
                        println!("Client disconnected");
                        break;
                    }
                    _ => {}
                }
            }
        },
        &mut req,
        None,
    )
    .expect("failed to spawn websocket")
}

#[miko]
async fn main() {
    println!("WebSocket server on ws://localhost:8080/ws");
}
```

## 发送和接收消息

### 发送消息

```rust
#[get("/ws/chat")]
async fn chat_ws(mut req: Req) {
    spawn_ws_event(
        |mut io| async move {
            // 文本消息
            io.send("Hello").await.unwrap();

            // 格式化消息
            io.send(format!("Time: {}", get_time())).await.unwrap();

            // JSON 消息
            io.send(Json(MyData { value: 42 })).await.unwrap();
        },
        &mut req,
        None,
    ).unwrap()
}
```

### 接收消息

```rust
#[get("/ws/receive")]
async fn receive_ws(mut req: Req) {
    spawn_ws_event(
        |mut io| async move {
            while let Some(result) = io.recv().await {
                match result {
                    Ok(msg) if msg.is_text() => {
                        let text = msg.into_text().unwrap();
                        println!("Text: {}", text);
                    }
                    Ok(msg) if msg.is_binary() => {
                        let data = msg.into_data();
                        println!("Binary: {} bytes", data.len());
                    }
                    Ok(msg) if msg.is_close() => {
                        println!("Connection closed");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        },
        &mut req,
        None,
    ).unwrap()
}
```

## 分离读写

使用 `split()` 方法分离读写通道：

```rust
#[get("/ws/split")]
async fn split_ws(mut req: Req) {
    spawn_ws_event(
        |mut io| async move {
            io.send("Starting...").await.unwrap();

            let (mut writer, mut reader, _handle) = io.split();

            // 发送任务
            {
                let mut w = writer.clone();
                tokio::spawn(async move {
                    for i in 1..=10 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        let _ = w.send(format!("Tick {}", i).into_message()).await;
                    }
                });
            }

            // 接收任务
            tokio::spawn(async move {
                while let Some(msg) = reader.next().await {
                    if let Ok(m) = msg {
                        if m.is_text() {
                            let text = m.into_text().unwrap();
                            println!("Received: {}", text);
                            let _ = writer.send(format!("Got: {}", text).into_message()).await;
                        } else if m.is_close() {
                            break;
                        }
                    }
                }
            });
        },
        &mut req,
        None,
    ).unwrap()
}
```

## 实时推送示例

### 服务器时间推送

```rust
use std::time::{SystemTime, UNIX_EPOCH};

#[get("/ws/time")]
async fn time_ws(mut req: Req) {
    spawn_ws_event(
        |mut io| async move {
            io.send("Time service started").await.unwrap();

            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                if io.send(format!("Server time: {}", now)).await.is_err() {
                    break;  // 客户端断开
                }
            }
        },
        &mut req,
        None,
    ).unwrap()
}
```

## 下一步

- 📤 学习 [响应处理](响应处理.md) 的 SSE 功能
- 🔍 了解 [请求提取器](请求提取器.md) 的用法
- 💉 使用 [依赖注入](依赖注入.md) 管理WebSocket相关组件
