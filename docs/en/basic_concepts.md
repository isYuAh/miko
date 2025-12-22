# Basic Concepts

This chapter introduces the core concepts and architectural design of the Miko framework.

## Core Components

The Miko framework consists of the following core components:

```
Application
    ├── Router
    │   ├── Handler
    │   └── Middleware
    ├── Config
    └── DependencyContainer
```

### Application

`Application` is the entry point of the framework, responsible for:

- Managing application configuration
- Holding route services
- Starting the HTTP server

```rust
use miko::*;
use miko::macros::*;

let router = Router::new()
.get("/", handler);

// Using file configuration
// You can use
let app = Application::new_(router);
// Or use
let config = ApplicationConfig::load_().unwrap();
let app = Application::new(config, router);

// Run application
app.run().await.unwrap();
```

### Router

`Router` is responsible for route management and request dispatching:

- Registering routes and their handlers
- Managing global state
- Applying middleware layers

```rust
let router = Router::new()
.get("/users", list_users)
.post("/users", create_user)
.get("/users/{id}", get_user);
```

Router supports generic state parameters:

```rust
struct AppState {
    db: Database,
    cache: Cache,
}

let state = Arc::new(AppState { /* ... */ });
let router = Router::new()
.with_state(state)
.get("/", handler);
```

### Handler

A Handler is a function that processes requests. It can be any async function that meets the following conditions:

- Arguments implement `FromRequest` or `FromRequestParts` trait (Note: only the last argument can be `FromRequest`)
- Return type implements `IntoResponse` trait

```rust
// Simplest handler
async fn simple() -> &'static str {
    "Hello"
}

// Handler with parameters
async fn with_params(
    Path(id): Path<u32>,
    Json(data): Json<CreateUser>,
) -> Json<User> {
    // ...
}

// Handler returning Result
async fn fallible() -> AppResult<Json<User>> {
    // ...
}
```

## Request Processing Flow

Request processing in Miko follows this flow:

```
HTTP Request
    ↓
[Hyper Server]
    ↓
[Tower Middleware Stack]  ← Middleware Chain
    ↓
[Router]  ← Route Matching
    ↓
[Extract Parameters]  ← Extract Request Data
    ↓
[Handler]  ← Business Logic
    ↓
[IntoResponse]  ← Convert to Response
    ↓
[Tower Middleware Stack]  ← Response Middleware
    ↓
HTTP Response
```

### 1. Route Matching

The Router finds the corresponding handler based on the request method and path:

```rust
// Define routes
router
.get("/users", list_users)      // GET /users
.get("/users/{id}", get_user)   // GET /users/123
.post("/users", create_user);    // POST /users
```

Path parameters are extracted and stored in `PathParams`.

### 2. Parameter Extraction

Handler parameters are automatically extracted from the request:

```rust
async fn handler(
    Path(id): Path<u32>,           // Extract from path
    Query(params): Query<MyQuery>, // Extract from query string
    Json(body): Json<MyData>,      // Extract from request body
    State(state): State<AppState>, // Extract from global state
) -> impl IntoResponse {
    // ...
}
```

Extractors execute in the following order:

1. **FromRequestParts** - Does not consume the request body (Path, Query, Headers, etc.)
2. **FromRequest** - May consume the request body (Json, Form, Multipart, etc.)

> ⚠️ **Note**: Only one extractor can consume the request body!

### 3. Business Processing

The Handler executes business logic, which can:

- Access the database
- Call external services
- Process business rules
- Return results or errors

### 4. Response Conversion

The return value is converted into an HTTP response via the `IntoResponse` trait:

```rust
// Return string directly
async fn text_response() -> &'static str {
    "Hello"
}

// Return JSON
async fn json_response() -> Json<User> {
    Json(user)
}

// Return tuple (StatusCode + Data)
async fn with_status() -> (StatusCode, Json<User>) {
    (StatusCode::CREATED, Json(user))
}

// Return Result
async fn fallible() -> AppResult<Json<User>> {
    Ok(Json(user))
}
```

## Type System

### FromRequest

Extracts data from the complete request, consuming the request body:

```rust
pub trait FromRequest<S, M = ()>: Sized {
    fn from_request(req: Req, state: Arc<S>) -> FRFut<Self>;
}
```

Types implementing `FromRequest`:

- `Json<T>` - JSON request body
- `Form<T>` - Form data
- `Multipart` - File upload, get raw stream
- `MultipartResult` - File upload, get parsed structure (files stored as temporary files to prevent memory overflow,
  accessed via `MultipartFileDiskLinker`)
- `ValidatedJson<T>` - Validated JSON (using `garde`)

### FromRequestParts

Extracts data from parts of the request, without consuming the request body:

```rust
pub trait FromRequestParts<S, M = ()>: Sized {
    fn from_request_parts(parts: &mut Parts, state: Arc<S>) -> FRFut<Self>;
}
```

Types implementing `FromRequestParts`:

- `Path<T>` - Path parameters
- `Query<T>` - Query parameters
- `State<T>` - Global state
- `HeaderMap` - Request headers
- `Method` - HTTP method
- `Uri` - Request URI

### IntoResponse

Converts a type into an HTTP response:

```rust
pub trait IntoResponse {
    fn into_response(self) -> Response;
}
```

The framework provides implementations for the following types:

- Basic types: `&str`, `String`, `&[u8]`, `Vec<u8>`
- JSON: `Json<T>`
- HTML: `Html<T>`
- Tuples: `(StatusCode, T)`, `(HeaderMap, T)`
- Result: `Result<T, E>`

## Feature Configuration

Miko uses a modular design, controlling functionality via Cargo features:

```toml
[dependencies]
miko = { version = "0.3.5", features = ["full"] }
```

### Available Features

| Feature      | Description     | Includes                                      |
|--------------|-----------------|-----------------------------------------------|
| `full`       | All features    | All features below                            |
| `macro`      | Route macros    | `#[get]`, `#[post]` etc.                      |
| `auto`       | Auto features   | Auto route registration, dependency injection |
| `ext`        | Extensions      | CORS, static file service                     |
| `utoipa`     | OpenAPI         | API documentation generation                  |
| `validation` | Data validation | garde integration                             |

### Enable on Demand

```toml
# Enable only required features
miko = { version = "0.3.5", features = ["macro", "auto"] }
```

### Minimal Configuration

Using no features:

```toml
miko = "0.3.5"
```

At this point, only core functions are available, and routes must be manually registered:

```rust
let router = Router::new()
.route("/", Method::GET, handler);
```

## Async Runtime

Miko is built on the Tokio runtime and needs to run in an async context:

```rust
#[tokio::main]
async fn main() {
    let router = Router::new()
        .get("/", handler);

    Application::new_(router)
        .run()
        .await
        .unwrap();
}
```

All handlers are async functions:

```rust
async fn handler() -> &'static str {
    // Can execute async operations
    tokio::time::sleep(Duration::from_secs(1)).await;
    "Hello"
}
```

## Tower Ecosystem Integration

Miko is fully compatible with the [Tower](https://github.com/tower-rs/tower) ecosystem:

```rust
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;

router.service(
"/no_macro",
handler
.with_state(Arc::new(router.state.clone())) // Need to determine state first, convert to Service
.layer(layer), // Service can chain layer calls
); // Use Tower middleware
```

This allows you to use a large number of middleware from the Tower ecosystem, such as:

- `tower-http` - HTTP related middleware
- `tower-governor` - Rate limiting
- `tower-timeout` - Timeout control

See [Middleware and Layers](middleware_and_layers.md) for details.

## Error Handling

Miko provides a unified error handling mechanism:

```rust
use miko::{AppError, AppResult};

async fn handler() -> AppResult<Json<User>> {
    let user = get_user()
        .await
        .map_err(|e| AppError::NotFound(format!("User not found: {}", e)))?;

    Ok(Json(user))
}
```

The framework automatically converts errors into a standard JSON response format:

```json
{
  "status": 404,
  "error": "NOT_FOUND",
  "message": "404 Not Found",
  "trace_id": "trace-641d28a04dfe2-ThreadId3",
  "timestamp": 1761222374
}
```

See [Error Handling](error_handling.md) for details.

## Dependency Injection

Use `#[component]` and `#[dep]` to implement dependency injection:

```rust
#[component]
impl Database {
    async fn new() -> Self {
        Self::connect().await
    }
}

#[get("/users")]
async fn list_users(#[dep] db: Arc<Database>) -> Json<Vec<User>> {
    // Use the injected database instance
}
```

> **Requires `auto` feature**

See [Dependency Injection](dependency_injection.md) for details.

## Next Steps

- 📖 Learn detailed usage of [Routing System](routing_system.md)
- 🔍 Understand types and usage of [Request Extractors](request_extractors.md)
- 📤 Master various ways of [Response Handling](response_handling.md)
- ⚠️ Dive deeper into [Error Handling](error_handling.md) mechanism
