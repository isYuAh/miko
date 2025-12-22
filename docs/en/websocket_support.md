# WebSocket Support

Miko provides WebSocket support, making it easy to implement bi-directional real-time communication.

## Basic Usage

### Creating a WebSocket Route

```rust
use miko::*;
use miko::macros::*;
use miko::ws::server::{spawn_ws_event, IntoMessage};
use std::time::Duration;

#[get("/ws")]
async fn websocket_handler(mut req: Req) {
    spawn_ws_event(
        |mut io| async move {
            // Send welcome message
            io.send("Connected!").await.expect("send failed");

            // Loop to receive and send messages
            while let Some(msg) = io.recv().await {
                match msg {
                    Ok(message) if message.is_text() => {
                        let text = message.into_text().unwrap();
                        println!("Received: {}", text);

                        // Echo message
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

## Sending and Receiving Messages

### Sending Messages

```rust
#[get("/ws/chat")]
async fn chat_ws(mut req: Req) {
    spawn_ws_event(
        |mut io| async move {
            // Text message
            io.send("Hello").await.unwrap();

            // Formatted message
            io.send(format!("Time: {}", get_time())).await.unwrap();

            // JSON message
            io.send(Json(MyData { value: 42 })).await.unwrap();
        },
        &mut req,
        None,
    ).unwrap()
}
```

### Receiving Messages

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

## Splitting Read and Write

Use the `split()` method to separate read and write channels:

```rust
#[get("/ws/split")]
async fn split_ws(mut req: Req) {
    spawn_ws_event(
        |mut io| async move {
            io.send("Starting...").await.unwrap();

            let (mut writer, mut reader, _handle) = io.split();

            // Sender task
            {
                let mut w = writer.clone();
                tokio::spawn(async move {
                    for i in 1..=10 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        let _ = w.send(format!("Tick {}", i).into_message()).await;
                    }
                });
            }

            // Receiver task
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

## Real-time Push Example

### Server Time Push

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
                    break;  // Client disconnected
                }
            }
        },
        &mut req,
        None,
    ).unwrap()
}
```

## Next Steps

- 📤 Learn about SSE functionality in [Response Handling](response_handling.md).
- 🔍 Understand usage of [Request Extractors](request_extractors.md).
- 💉 Use [Dependency Injection](dependency_injection.md) to manage WebSocket-related components.
