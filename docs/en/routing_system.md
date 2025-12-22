# Routing System

Miko provides a flexible and powerful routing system, supporting route macros, manual registration, path parameters,
nested routes, and more.

## Route Definition

### Using Route Macros (Recommended)

Use macros like `#[get]`, `#[post]` to define routes, and use `#[miko]` for automatic registration:

> **Requires `macro` feature, automatic registration requires `auto` feature**

```rust
use miko::*;
use miko::macros::*;

#[get("/")]
async fn index() -> &'static str {
    "Home"
}

#[get("/users")]
async fn list_users() -> &'static str {
    "User list"
}

#[post("/users")]
async fn create_user() -> &'static str {
    "User created"
}

#[miko]
async fn main() {
    // Routes automatically collected and registered
}
```

### Manual Route Registration (Note: State is injected into handler during route, so route macros don't support state, need with_state before mounting routes)

If you don't use macros, you can register routes manually:

```rust
use miko::*;

async fn index() -> &'static str {
    "Home"
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .get("/", index)
        .post("/users", create_user)
        .put("/users/{id}", update_user)
        .delete("/users/{id}", delete_user);

    let config = ApplicationConfig::default();
    Application::new(config, router).run().await.unwrap();
}
```

### Custom Methods and Paths

Use `#[route]` macro to customize HTTP methods (others can also extend methods, e.g. `#[get("/", method="post,put")]`):

```rust
use hyper::Method;

// Using macro
#[route("/api/data", method = "get")]
async fn get_data() -> &'static str {
    "Data"
}

// Or manual registration
router.route(Method::PATCH, "/api/data", patch_handler);
```

## Available Route Macros

Miko provides macros for all common HTTP methods:

| Macro        | HTTP Method | Usage                      |
|--------------|-------------|----------------------------|
| `#[get]`     | GET         | Retrieve resource          |
| `#[post]`    | POST        | Create resource            |
| `#[put]`     | PUT         | Update resource (full)     |
| `#[patch]`   | PATCH       | Update resource (partial)  |
| `#[delete]`  | DELETE      | Delete resource            |
| `#[head]`    | HEAD        | Retrieve meta-information  |
| `#[options]` | OPTIONS     | Retrieve supported methods |

All macros support parameter annotation features of `#[route]`.

## Path Parameters

### Defining Path Parameters

Use `{param_name}` to define path parameters:

```rust
#[get("/users/{id}")]
async fn get_user(Path(id): Path<u32>) -> String {
    format!("User ID: {}", id)
}

#[get("/posts/{post_id}/comments/{comment_id}")]
async fn get_comment(
    Path(post_id): Path<u32>,
    Path(comment_id): Path<u32>
) -> String {
    format!("Post: {}, Comment: {}", post_id, comment_id)
}
```

### Using `#[path]` Annotation

Using parameter annotations with macros is more concise:

```rust
#[get("/users/{id}")]
async fn get_user(#[path] id: u32) -> String {
    format!("User ID: {}", id)
}

#[get("/users/{user_id}/posts/{post_id}")]
async fn get_user_post(
    #[path] user_id: u32,
    #[path] post_id: u32,
) -> String {
    format!("User {}, Post {}", user_id, post_id)
}
```

### Type Safety

Path parameters support any type that implements `FromStr`:

```rust
use uuid::Uuid;

#[get("/items/{id}")]
async fn get_item(#[path] id: Uuid) -> String {
    format!("Item UUID: {}", id)
}

#[get("/products/{slug}")]
async fn get_product(#[path] slug: String) -> String {
    format!("Product: {}", slug)
}
```

If type conversion fails, a 400 Bad Request error is automatically returned.

## Nested Routes

### Using

`nest` Method (Note: State is inherited from original router, in other words state is injected into handler during
route)

Add a unified prefix to a group of routes:

```rust
use miko::*;
use miko::macros::*;

// API v1 routes
#[get("/users")]
async fn v1_users() -> &'static str { "API v1 users" }

#[get("/posts")]
async fn v1_posts() -> &'static str { "API v1 posts" }

// API v2 routes
#[get("/users")]
async fn v2_users() -> &'static str { "API v2 users" }

#[tokio::main]
async fn main() {
    let v1_router = Router::new()
        .get("/users", v1_users)
        .get("/posts", v1_posts);

    let v2_router = Router::new()
        .get("/users", v2_users);

    let router = Router::new()
        .nest("/api/v1", v1_router)
        .nest("/api/v2", v2_router);

    // Access /api/v1/users, /api/v1/posts, /api/v2/users

    let config = ApplicationConfig::default();
    Application::new(config, router).run().await.unwrap();
}
```

### Using `merge` Method

Merge two routers:

```rust
let user_router = Router::new()
.get("/users", list_users)
.get("/users/{id}", get_user);

let post_router = Router::new()
.get("/posts", list_posts)
.get("/posts/{id}", get_post);

let router = Router::new()
.merge(user_router)
.merge(post_router);
```

### Modular Routes

It is recommended to organize routes by functional modules:

```rust
// src/routes/users.rs
use miko::*;
use miko::macros::*;

#[get("/users")]
pub async fn list() -> &'static str { "Users" }

#[get("/users/{id}")]
pub async fn get(#[path] id: u32) -> String {
    format!("User {}", id)
}

pub fn router() -> Router {
    Router::new()
        .get("/users", list)
        .get("/users/{id}", get)
}

// src/routes/posts.rs
use miko::*;
use miko::macros::*;

#[get("/posts")]
pub async fn list() -> &'static str { "Posts" }

pub fn router() -> Router {
    Router::new()
        .get("/posts", list)
}

// src/main.rs
mod routes;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .merge(routes::users::router())
        .merge(routes::posts::router());

    let config = ApplicationConfig::default();
    Application::new(config, router).run().await.unwrap();
}
```

## Module Routes and Prefixes

### #[prefix] Macro

Use `#[prefix]` macro to add a path prefix to all routes in a module:

```rust
use miko::*;
use miko::macros::*;

#[prefix("/api")]
mod api {
    #[get("/users")]
    async fn list_users() -> &'static str {
        "users"  // Actual path: GET /api/users
    }

    #[get("/posts")]
    async fn list_posts() -> &'static str {
        "posts"  // Actual path: GET /api/posts
    }
}

#[prefix("/admin")]
mod admin {
    #[get("/users")]
    async fn admin_users() -> &'static str {
        "admin users"  // Actual path: GET /admin/users
    }
}

#[miko]
async fn main() {
    // Routes in modules automatically prefixed
}
```

### Nested Prefixes

`#[prefix]` supports nesting:

```rust
#[prefix("/api")]
mod api {
    #[prefix("/v1")]
    mod v1 {
        #[get("/users")]
        async fn list_users() -> &'static str {
            "v1 users"  // Actual path: GET /api/v1/users
        }
    }

    #[prefix("/v2")]
    mod v2 {
        #[get("/users")]
        async fn list_users() -> &'static str {
            "v2 users"  // Actual path: GET /api/v2/users
        }
    }
}
```

### Difference between prefix and nest

| Feature        | `#[prefix]`                 | `Router::nest`                              |
|----------------|-----------------------------|---------------------------------------------|
| Usage Location | Module level (Compile time) | Router level (Runtime)                      |
| Path Handling  | Simple prefix concatenation | True route nesting, modifies internal paths |
| Usage Scenario | Modular code organization   | Dynamic route composition                   |

```rust
// #[prefix] - Compile time prefix
#[prefix("/api")]
mod api {
    #[get("/users")]  // Compiled to: GET /api/users
    async fn users() {}
}

// nest - Runtime nesting
let api_router = Router::new()
.get("/users", users);

let router = Router::new()
.nest("/api", api_router);  // Runtime: GET /api/users
```

### Combining Usage

`#[prefix]` can be used with `#[layer]`:

```rust
use tower_http::timeout::TimeoutLayer;
use std::time::Duration;

#[prefix("/api")]
#[layer(TimeoutLayer::new(Duration::from_secs(30)))]
mod api {
    #[get("/users")]
    async fn users() -> &'static str {
        "users"  // GET /api/users, with 30s timeout
    }
}
```

## Route Priority

Route matching prioritizes static routes:

```rust
let router = Router::new()
.get("/users/me", get_current_user)      // Registered first (concrete path)
.get("/users/{id}", get_user);           // Registered later (parameter path)
```

Both `/users/me` and `/users/{id}` match, but `me` is static route and prioritized.

## Wildcard Routes

Matches any path segment:

```rust
// Matches all paths after /files/
#[get("/files/{*path}")]
async fn serve_file(Path(path): Path<String>) -> String {
    format!("Serving file: {}", path)
}

// Access /files/docs/guide.md
// path = "docs/guide.md"
```

## Route Groups and Middleware

Apply the same middleware to a group of routes (Effectively applied during merge, nest, into_tower_service, so timing is
flexible):

```rust
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

let protected_router = Router::new()
.get("/admin/users", admin_users)
.get("/admin/settings", admin_settings)
.with_layer(auth_middleware());  // Applied only to these routes

let public_router = Router::new()
.get("/", index)
.get("/about", about);

let router = Router::new()
.merge(public_router)
.merge(protected_router)
.layer(TraceLayer::new_for_http());  // Applied to all routes
```

See [Middleware and Layers](middleware_and_layers.md).

## Route Handlers

### Handler Signature

A Handler is any async function that meets these conditions:

- Arguments implement `FromRequest` or `FromRequestParts`
- Return type implements `IntoResponse`

```rust
// No args
async fn handler() -> &'static str { "Hello" }

// Single arg
async fn with_json(Json(data): Json<MyData>) -> Json<Response> {
    // ...
}

// Multiple args
async fn complex_handler(
    Path(id): Path<u32>,
    Query(params): Query<HashMap<String, String>>,
    Json(body): Json<CreateData>,
    State(db): State<Database>,
) -> AppResult<Json<Response>> {
    // ...
}
```

### Parameter Extraction Order

There are two types of extractors:

1. **FromRequestParts** - Does not consume request body (Path, Query, Headers, State, etc.)
2. **FromRequest** - May consume request body (Json, Form, Multipart, etc.)

You can have multiple `FromRequestParts` extractors, but only one `FromRequest` extractor:

```rust
// ✅ Correct
async fn handler(
    Path(id): Path<u32>,           // FromRequestParts
    Query(q): Query<MyQuery>,       // FromRequestParts
    headers: HeaderMap,             // FromRequestParts
    Json(body): Json<MyData>,       // FromRequest
) {}

// ❌ Error - Cannot have two FromRequest
async fn bad_handler(
    Json(body1): Json<Data1>,       // FromRequest
    Json(body2): Json<Data2>,       // FromRequest - Compilation error!
) {}
```

### Return Types

Our route related type definitions are as follows:

```rust
pub type RespBody = BoxBody<Bytes, MikoError>; // Updated to MikoError
pub type Resp = Response<RespBody>;
pub type Req = Request<ReqBody>; // ReqBody uses Infallible (or MikoError depending on version)
```

Handler can return any type that implements `IntoResponse`:

```rust
// String
async fn text() -> &'static str { "Hello" }

// JSON
async fn json() -> Json<User> { Json(user) }

// With status code
async fn created() -> (StatusCode, Json<User>) {
    (StatusCode::CREATED, Json(user))
}

// Result
async fn fallible() -> AppResult<Json<User>> {
    Ok(Json(user))
}

// Custom response
async fn custom() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header("X-Custom", "value")
        .body(Full::new(Bytes::from("Hello")).map_err(Into::into).boxed())
        .unwrap()
}
```

See [Response Handling](response_handling.md).

## Complete Example

A complete example demonstrating various routing features:

```rust
use miko::{*, extractor::{Json, Path, Query}};
use miko::macros::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MyQuery {
    id: u32,
    name: String,
}

// Basic route
#[get("/")]
async fn index() -> &'static str {
    "Welcome"
}

// Path params
#[get("/users/{id}")]
async fn get_user(#[path] id: u32) -> Json<User> {
    Json(User { id, name: format!("User {}", id) })
}

// Query params
#[get("/search")]
async fn search(Query(params): Query<MyQuery>) -> String {
    format!("Query: {:?}", params)
}

// POST request
#[post("/users")]
async fn create_user(Json(user): Json<User>) -> (StatusCode, Json<User>) {
    (StatusCode::CREATED, Json(user))
}

// Multiple path params
#[get("/users/{user_id}/posts/{post_id}")]
async fn get_user_post(
    #[path] user_id: u32,
    #[path] post_id: u32,
) -> String {
    format!("User {}, Post {}", user_id, post_id)
}

#[miko]
async fn main() {
    println!("🚀 Server running on http://localhost:8080");
}
```

## Next Steps

- 🔍 Learn detailed usage of [Request Extractors](request_extractors.md)
- 📤 Understand various ways of [Response Handling](response_handling.md)
- 🔐 Use [Middleware and Layers](middleware_and_layers.md) to add authentication etc.
