# Middleware and Layers

Miko provides middleware functionality based on Tower's `Layer` mechanism.

## Core Concepts

### Lazy Application of Layers

Layers configured on a `Router` are **not applied immediately**. They are applied at the following times:

- When `into_tower_service()` is called.
- When `merge()` is called to merge routers.
- When `nest()` is called to nest routers.

```rust
use miko::*;
use miko::macros::*;
use tower_http::timeout::TimeoutLayer;
use std::time::Duration;

let mut router = Router::new();
router.with_layer(TimeoutLayer::new(Duration::from_secs(30)));  // Register Layer
// ⚠️ Layer is not yet applied at this point

// ✅ Layer is applied here
let svc = router.into_tower_service();
```

## Router-level Layers

### `with_layer` Method

Use `with_layer` to add middleware to the entire `Router`:

```rust
use miko::*;
use miko::macros::*;
use tower_http::{
    trace::TraceLayer,
    timeout::TimeoutLayer,
    compression::CompressionLayer,
};
use std::time::Duration;

#[get("/api/users")]
async fn users() -> &'static str {
    "users"
}

#[miko]
async fn main() {
    let mut router = Router::new();

    // Chain multiple Layers
    router
        .with_layer(TraceLayer::new_for_http())
        .with_layer(TimeoutLayer::new(Duration::from_secs(30)))
        .with_layer(CompressionLayer::new());
}
```

### Using `ServiceBuilder`

Tower's `ServiceBuilder` can combine multiple Layers:

```rust
use tower::ServiceBuilder;
use tower_http::{trace::TraceLayer, compression::CompressionLayer};
use std::time::Duration;

#[miko]
async fn main() {
    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .timeout(Duration::from_secs(30));

    let mut router = Router::new();
    router.with_layer(middleware);
}
```

## Handler-level Layers

### `WithState` Trait

The `WithState` trait allows a handler to immediately obtain state and be wrapped as a `Service`, followed by chained
`LayerExt` calls:

```rust
use miko::*;
use miko::macros::*;
use miko::endpoint::{WithState, LayerExt};
use tower_http::timeout::TimeoutLayer;
use std::time::Duration;
use std::sync::Arc;

struct AppState {
    db: Database,
}

async fn get_user(State(state): State<Arc<AppState>>) -> String {
    format!("User from {:?}", state.db)
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState { db: Database::new() });

    // Recommended: Use router.state.clone()
    let endpoint = get_user
        .with_state(state.clone())  // Immediately provide state and wrap as Service
        .layer(TimeoutLayer::new(Duration::from_secs(30)));  // Chain layer calls

    let router = Router::with_state(state)
        .get_service("/user", endpoint);
}
```

### `LayerExt` Trait

`LayerExt` provides a chained `.layer()` method for `Service`s:

```rust
use miko::endpoint::LayerExt;
use miko::macros::*;
use tower_http::{timeout::TimeoutLayer, compression::CompressionLayer};
use std::time::Duration;

async fn handler() -> String {
    "Hello".to_string()
}

#[tokio::main]
async fn main() {
    let router = Router::new();
    let state = router.state.clone();

    // Chain multiple layers
    let endpoint = handler
        .with_state(state)
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(CompressionLayer::new());

    router.get_service("/api", endpoint);
}
```

## CORS Support

### `cors_any` Method

> **Requires `ext` feature**

The framework provides a `cors_any()` method to quickly enable CORS (useful for development):

```rust
use miko::*;
use miko::macros::*;

#[miko]
async fn main() {
    let mut router = Router::new();

    // Allow all origins (equivalent to CorsLayer::permissive)
    router.cors_any();
}
```

### Custom CORS

Use `tower-http`'s `CorsLayer` (**no** `ext` feature required):

```rust
use tower_http::cors::CorsLayer;
use http::{Method, HeaderValue};

#[miko]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST]);

    let mut router = Router::new();
    router.with_layer(cors);
}
```

## Module-level Layers

Use the `#[layer]` macro to add middleware to all routes within a module:

```rust
use tower_http::timeout::TimeoutLayer;
use std::time::Duration;

#[layer(TimeoutLayer::new(Duration::from_secs(30)))]
mod api {
    #[get("/users")]
    async fn list_users() -> &'static str {
        "users"  // Actual path: GET /api/users, with 30s timeout
    }

    #[get("/posts")]
    async fn list_posts() -> &'static str {
        "posts"  // Actual path: GET /api/posts, with 30s timeout
    }
}
```

## Function-level Layers

Use the `#[layer]` macro to add middleware to a single function:

```rust
use tower_http::timeout::TimeoutLayer;
use tower_http::compression::CompressionLayer;
use std::time::Duration;

// Single layer
#[get("/users")]
#[layer(TimeoutLayer::new(Duration::from_secs(30)))]
async fn list_users() -> &'static str {
    "users"
}

// Multiple layers (Declared top-to-bottom, applied inner-to-outer)
#[post("/data")]
#[layer(TimeoutLayer::new(Duration::from_secs(30)))]
#[layer(CompressionLayer::new())]
async fn process_data() -> &'static str {
    // Execution chain: CompressionLayer -> TimeoutLayer -> handler
    "processed"
}
```

## Declarative Middleware

Miko provides the `#[middleware]` macro, allowing you to define middleware just like writing a normal Handler.

### Basic Usage

Mark an async function with `#[middleware]`, and it will automatically get `_req` (request) and `_next` (next
middleware) variables.

```rust
use miko::*;
use miko::macros::*;

#[middleware]
async fn simple_logger() -> AppResult<Resp> {
    println!("Before request");
    let resp = _next.run(_req).await?;
    println!("After request");
    Ok(resp)
}

#[get("/")]
#[layer(simple_logger())] // Apply middleware
async fn hello() -> &'static str {
    "Hello"
}
```

### Argument Injection

Middleware functions support the following types of arguments:

1. **Explicit `Req` and `Next`**: If you want to customize variable names.
2. **Normal Arguments**: Passed when applying the middleware.
3. **Dependency Injection (`#[dep]`)**: Automatically injected from the dependency container.
4. **Configuration Injection (`#[config]`)**: Automatically injected from the configuration file.

```rust
#[middleware]
async fn auth_guard(
    // Normal argument, passed in #[layer(auth_guard(role))]
    required_role: String,
    // Dependency injection
    #[dep] db: Arc<Database>,
    // Config injection
    #[config("jwt.secret")] secret: String,
    // Explicit Req and Next (optional)
    req: Req,
    next: Next,
) -> AppResult<Resp> {
    // Check permissions...
    if !check_permission(&db, &secret, &required_role) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    next.run(req).await
}

// Use middleware
#[get("/admin")]
#[layer(auth_guard("admin".to_string()))]
async fn admin_panel() -> &'static str {
    "Admin Content"
}
```

### Capturing Context Variables

Declarative middleware automatically captures external variables (via Clone), making argument passing very convenient.

```rust
#[middleware]
async fn with_id(id: i32) -> AppResult<Resp> {
    println!("ID is {}", id);
    _next.run(_req).await
}

#[get("/")]
#[layer(with_id(10086))]
async fn index() -> &'static str {
    "ok"
}
```

## Tower Middleware Compatibility

Miko is fully compatible with middleware from the Tower ecosystem, including those that modify the Body type (like
`CompressionLayer`) or return errors (like `TimeoutLayer`).

### Supported Middleware Types

- **Infallible Middleware**: Middleware that does not return errors (e.g., `TraceLayer`).
- **Fallible Middleware**: Middleware that may return errors (e.g., `TimeoutLayer`, `RateLimitLayer`).
    - Miko automatically captures these errors and converts them into `AppError`, eventually generating a unified JSON
      error response (or a response conforming to HTTP semantics).
- **Body Modifying Middleware**: Middleware that modifies the response body (e.g., `CompressionLayer`).
    - Miko automatically adapts to different Body types.

### Example: Timeout and Compression

```rust
use tower_http::{timeout::TimeoutLayer, compression::CompressionLayer};
use std::time::Duration;

#[miko]
async fn main() {
    let mut router = Router::new();

    router
        // Automatically handle Gzip compression
        .with_layer(CompressionLayer::new())
        // Automatically handle timeout (throws error -> 500/504 response)
        .with_layer(TimeoutLayer::new(Duration::from_secs(5)));
}
```

### `ServiceBuilder`

Tower's `ServiceBuilder` can be used to easily combine multiple middlewares:

```rust
use tower::ServiceBuilder;
use tower_http::{
    trace::TraceLayer,
    compression::CompressionLayer,
    timeout::TimeoutLayer,
};
use std::time::Duration;

#[miko]
async fn main() {
    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::new(Duration::from_secs(30)));

    let mut router = Router::new();
    router.with_layer(middleware);
}
```

## Next Steps

- 🔍 Learn about [Error Handling](error_handling.md) for unified error formats.
- 📖 Study [Response Handling](response_handling.md) to construct responses.
- 🚀 Check [Advanced Features](advanced_features.md) for more functionality.
