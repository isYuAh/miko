# Response Handling

Miko provides a flexible response handling mechanism through the `IntoResponse` trait, allowing you to build HTTP
responses in various ways.

## IntoResponse Trait

Any type that implements `IntoResponse` can be used as a return value for a Handler:

```rust
pub trait IntoResponse {
    fn into_response(self) -> Response;
}
```

The framework provides implementations for common types, and you can also implement this trait for your custom types.

## Basic Response Types

### String Response

The simplest way to respond:

```rust
#[get("/text")]
async fn text() -> &'static str {
    "Hello, World!"
}

#[get("/owned")]
async fn owned_text() -> String {
    format!("Generated at: {}", chrono::Utc::now())
}
```

Default Content-Type is `text/plain`.

### JSON Response

Return JSON data:

```rust
use miko::{*, extractor::Json};
use miko::macros::*;
use serde::Serialize;

#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[get("/users/{id}")]
async fn get_user(#[path] id: u32) -> Json<User> {
    Json(User {
        id,
        name: "Alice".into(),
        email: "alice@example.com".into(),
    })
}
```

Automatically sets `Content-Type: application/json`.

### HTML Response

Return HTML content:

```rust
use miko::http::response::into_response::Html;

#[get("/page")]
async fn page() -> Html {
    Html("<html><body><h1>Hello</h1></body></html>".into())
}

#[get("/template")]
async fn template() -> Html {
    let content = format!(
        r#"\
        <!DOCTYPE html>\
        <html>\
        <head><title>My Page</title></head>\
        <body>\
            <h1>Welcome</h1>\
            <p>Current time: {}</p>\
        </body>\
        </html>\
        "#,
        chrono::Utc::now()
    );
    Html(content)
}
```

Automatically sets `Content-Type: text/html; charset=utf-8`.

## Status Codes

### Using Tuples to Return Status Codes

```rust
use hyper::StatusCode;

// 201 Created
#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> (StatusCode, Json<User>) {
    let user = create_user_in_db(data);
    (StatusCode::CREATED, Json(user))
}

// 204 No Content
#[delete("/users/{id}")]
async fn delete_user(#[path] id: u32) -> StatusCode {
    delete_user_from_db(id);
    StatusCode::NO_CONTENT
}

// Custom Status Code
#[get("/custom")]
async fn custom() -> (StatusCode, &'static str) {
    (StatusCode::IM_A_TEAPOT, "I'm a teapot!")
}
```

### Common Status Codes

```rust
StatusCode::OK                    // 200
StatusCode::CREATED               // 201
StatusCode::ACCEPTED              // 202
StatusCode::NO_CONTENT            // 204
StatusCode::MOVED_PERMANENTLY     // 301
StatusCode::FOUND                 // 302
StatusCode::BAD_REQUEST           // 400
StatusCode::UNAUTHORIZED          // 401
StatusCode::FORBIDDEN             // 403
StatusCode::NOT_FOUND             // 404
StatusCode::INTERNAL_SERVER_ERROR // 500
```

## Response Headers

### Using Tuples to Add Response Headers

```rust
use hyper::HeaderMap;

#[get("/with-headers")]
async fn with_headers() -> (HeaderMap, Json<User>) {
    let mut headers = HeaderMap::new();
    headers.insert("X-Custom-Header", "value".parse().unwrap());
    headers.insert("X-Request-Id", "123456".parse().unwrap());

    (headers, Json(user))
}
```

### Combining Status Code and Headers

```rust
#[post("/users")]
async fn create_with_headers() -> (StatusCode, HeaderMap, Json<User>) {
    let mut headers = HeaderMap::new();
    headers.insert("Location", "/users/123".parse().unwrap());

    (StatusCode::CREATED, headers, Json(user))
}
```

## Result Types

### Using AppResult

It is recommended to use the `AppResult` type provided by the framework:

```rust
use miko::{AppResult, AppError};

#[get("/users/{id}")]
async fn get_user(#[path] id: u32) -> AppResult<Json<User>> {
    let user = db.find_user(id)
        .ok_or(AppError::NotFound(format!("User {} not found", id)))?;

    Ok(Json(user))
}

#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> AppResult<Json<User>> {
    // Validation
    if data.email.is_empty() {
        return Err(AppError::BadRequest("Email is required".into()));
    }

    // Check duplicate
    if db.email_exists(&data.email) {
        return Err(AppError::Conflict("Email already exists".into()));
    }

    let user = db.create_user(data)?;
    Ok(Json(user))
}
```

Errors are automatically converted to JSON responses. See [Error Handling](error_handling.md) for details.

### Result with Status Codes

```rust
#[post("/users")]
async fn create_user(
    Json(data): Json<CreateUser>
) -> AppResult<(StatusCode, Json<User>)> {
    let user = db.create_user(data)?;
    Ok((StatusCode::CREATED, Json(user)))
}
```

## Server-Sent Events (SSE)

Real-time data streaming to clients:

> **SSE requires the `sse` parameter in the `#[miko]` macro**: `#[miko(sse)]`
> You can omit it, but client-side connection closure might cause panics and excessive log output.
> The `sse` parameter applies a panic hook to ignore such errors.
> If you are not using `#[miko]`, you can use `set_sse_panic_hook()` manually.

```rust
use miko::http::response::sse::SseSender;
use std::time::Duration;

// We implemented IntoResponse for closures receiving SseSender, or you can use spawn_sse_event()
#[get("/events")]
async fn events() {
    |sender: SseSender| async move {
        // Send message (using IntoMessage trait)
        sender.send("Connected").await.or_break();

        // Periodic push
        for i in 1..=10 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            sender.send(format!("Event {}", i)).await.or_break();
        }

        // Send JSON
        sender.send(Json(MyData { value: 42 })).await.or_break();

        sender.send("Done").await.or_break();
    }
}
```

### SSE Event Format

```rust
use miko::http::response::sse::{SseEvent, SseSender};

#[get("/custom-events")]
async fn custom_events() {
    |sender: SseSender| async move {
        // Custom event
        sender.event(
            "greeting", // event name
            SseEvent::data("Hello") // also IntoMessage
                .id("msg-1")
                .retry(3000)
        ).await.or_break();

        // Simple message
        sender.send("Simple message").await.or_break();
    }
}
```

### Client Example

```javascript
const eventSource = new EventSource('/events');

eventSource.onmessage = (event) => {
  console.log('Received:', event.data);
};

eventSource.addEventListener('greeting', (event) => {
  console.log('Greeting:', event.data);
});

eventSource.onerror = (error) => {
  console.error('Error:', error);
  eventSource.close();
};
```

### Disconnection Handling

Use `.or_break()` to exit gracefully when a client disconnects (intercepted by the aforementioned panic hook):

```rust
#[get("/stream")]
async fn stream() {
    |sender: SseSender| async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            // or_break() terminates the task if the client disconnects
            sender.send("data").await.or_break();
        }
    }
}
```

## File Responses

### Streaming File Download

Use streaming responses to avoid loading the entire file into memory:

```rust
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use miko_core::fallible_stream_body::FallibleStreamBody;
use hyper::Response;
use bytes::Bytes;

#[get("/download/{filename}")]
async fn download_file(#[path] filename: String) -> AppResult<Response<BoxBody<Bytes, MikoError>>> {
    let path = format!("./uploads/{}", filename);
    let file = File::open(&path).await?;
    let metadata = file.metadata().await?;

    let stream = ReaderStream::new(file);
    let body = FallibleStreamBody::with_size_hint(stream, metadata.len());

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/octet-stream")
        .header("Content-Disposition", format!("attachment; filename=\"{}\"", filename))
        .body(body.map_err(Into::into).boxed())
        .unwrap())
}
```

### Small File Responses

Small files can be read directly:

```rust
use bytes::Bytes;
use http_body_util::Full;

#[get("/image/{id}")]
async fn get_image(#[path] id: u32) -> AppResult<Response<BoxBody<Bytes, MikoError>>> {
    let data = tokio::fs::read(format!("./images/{}.jpg", id)).await?;

    Ok(Response::builder()
        .header("Content-Type", "image/jpeg")
        .body(Full::new(Bytes::from(data)).map_err(Into::into).boxed())
        .unwrap())
}
```

> **Tip**: For full static file service functionality, we recommend using `StaticSvc`.
> See [Advanced Features - Static File Service](advanced_features.md#static-file-service) for details.

## Redirection

```rust
use hyper::StatusCode;

#[get("/old-path")]
async fn redirect() -> (StatusCode, HeaderMap, &'static str) {
    let mut headers = HeaderMap::new();
    headers.insert("Location", "/new-path".parse().unwrap());

    (StatusCode::MOVED_PERMANENTLY, headers, "Redirecting...")
}

// Or use FOUND (302)
#[get("/temp-redirect")]
async fn temp_redirect() -> (StatusCode, HeaderMap) {
    let mut headers = HeaderMap::new();
    headers.insert("Location", "/new-location".parse().unwrap());

    (StatusCode::FOUND, headers)
}
```

## Custom Response Types

Implement `IntoResponse` for your own types:

```rust
use miko::http::response::into_response::IntoResponse;
use hyper::{Response, StatusCode};
use bytes::Bytes;
use http_body_util::Full;

struct ApiResponse<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Resp {
        let body = serde_json::json!({
            "code": self.code,
            "message": self.message,
            "data": self.data,
        });

        let bytes = serde_json::to_vec(&body).unwrap();

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(bytes)).map_err(Into::into).boxed())
            .unwrap()
    }
}

// Using custom response
#[get("/api/users/{id}")]
async fn get_user(#[path] id: u32) -> ApiResponse<User> {
    ApiResponse {
        code: 0,
        message: "Success".into(),
        data: Some(user),
    }
}
```

## Empty Responses

### Returning Status Code (No Content)

Returning a `StatusCode` directly generates an empty response body:

```rust
// 204 No Content
#[delete("/users/{id}")]
async fn delete_user(#[path] id: u32) -> StatusCode {
    // Perform deletion
    StatusCode::NO_CONTENT
}

// 202 Accepted
#[post("/tasks")]
async fn create_task(Json(data): Json<Task>) -> StatusCode {
    // Submit async task
    StatusCode::ACCEPTED
}
```

### Returning Unit Type

```rust
// Returning unit type - defaults to 200 OK
#[post("/notify")]
async fn notify() -> () {
    // Send notification
}
```

## Response Builder

Use Hyper's response builder:

```rust
use hyper::Response;
use bytes::Bytes;
use http_body_util::Full;

#[get("/custom")]
async fn custom_response() -> Response<BoxBody<Bytes, MikoError>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("X-Custom", "value")
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Custom response")).map_err(Into::into).boxed())
        .unwrap()
}
```

## Conditional Responses

Return different responses based on conditions:

```rust
#[get("/users/{id}")]
async fn get_user_conditional(
    #[path] id: u32,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = db.find_user(id);

    match user {
        Some(u) if headers.contains_key("X-Include-Details") => {
            // Return detailed info
            Json(UserDetail::from(u)).into_response()
        }
        Some(u) => {
            // Return basic info
            Json(UserBasic::from(u)).into_response()
        }
        None => {
            // Return 404
            StatusCode::NOT_FOUND.into_response()
        }
    }
}
```

## Complete Example

```rust
use miko::{*, extractor::{Json, Path},
           use miko::macros::*;
           http::response::into_response::Html};
use serde::{Deserialize, Serialize};
use hyper::{StatusCode, HeaderMap};

#[derive(Serialize, Deserialize)]
struct Post {
    id: u32,
    title: String,
    content: String,
}

#[derive(Deserialize)]
struct CreatePost {
    title: String,
    content: String,
}

// Text response
#[get("/")]
async fn index() -> &'static str {
    "Welcome to Blog API"
}

// HTML response
#[get("/page")]
async fn page() -> Html {
    Html("<h1>My Blog</h1>".into())
}

// JSON response
#[get("/posts")]
async fn list_posts() -> Json<Vec<Post>> {
    Json(vec![
        Post { id: 1, title: "First".into(), content: "...".into() }
    ])
}

// JSON with status code
#[post("/posts")]
async fn create_post(
    Json(data): Json<CreatePost>
) -> (StatusCode, Json<Post>) {
    let post = Post {
        id: 1,
        title: data.title,
        content: data.content,
    };
    (StatusCode::CREATED, Json(post))
}

// Result type
#[get("/posts/{id}")]
async fn get_post(#[path] id: u32) -> AppResult<Json<Post>> {
    let post = db.find_post(id)
        .ok_or(AppError::NotFound("Post not found".into()))?;
    Ok(Json(post))
}

// With headers
#[put("/posts/{id}")]
async fn update_post(
    #[path] id: u32,
    Json(data): Json<CreatePost>,
) -> AppResult<(HeaderMap, Json<Post>)> {
    let post = db.update_post(id, data)?;

    let mut headers = HeaderMap::new();
    headers.insert("X-Updated-At", chrono::Utc::now().to_string().parse().unwrap());

    Ok((headers, Json(post)))
}

// Delete - No Content
#[delete("/posts/{id}")]
async fn delete_post(#[path] id: u32) -> AppResult<StatusCode> {
    db.delete_post(id)?;
    Ok(StatusCode::NO_CONTENT)
}

// SSE Example
#[get("/events")]
async fn events() {
    |sender: SseSender| async move {
        for i in 1..=5 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            sender.send(format!("Event {}", i)).await.or_break();
        }
    }
}

#[miko(sse)]  // Enable SSE support
async fn main() {
    println!("🚀 Blog API running");
}
```

## Next Steps

- ⚠️ Learn [Error Handling](error_handling.md) mechanism
- 🔍 Understand usage of [Request Extractors](request_extractors.md)
- 🌐 Explore [WebSocket](websocket_support.md) for bi-directional communication
