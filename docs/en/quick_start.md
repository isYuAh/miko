# Quick Start

This tutorial will help you get started with the Miko framework in 5 minutes and create your first Web application.

## Prerequisites

- Rust 1.75 or higher
- Cargo package manager

## Create a New Project

```bash
car go new my-miko-app
cd my-miko-app
```

## Add Dependencies

Edit your `Cargo.toml` file:

```toml
[dependencies]
# Use default features (includes macros, auto-registration, extension features)
miko = "0.3"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }

# Or enable all features (including OpenAPI and data validation)
# miko = { version = "0.3", features = ["full"] }
```

> **Feature Flags**
> - `default`: Core features (`macro` + `auto` + `ext`), **enabled by default**
> - `full`: Enable all features (including `utoipa` and `validation`)
> - When `utoipa` or `validation` is enabled, you don't need to manually add these dependencies; the framework
    re-exports them automatically.
> - See [Basic Concepts](basic_concepts.md#features) for more details.

## Hello World

Create the simplest application. Edit `src/main.rs`:

```rust
use miko::*
use miko::macros::*

// Define a simple handler function
#[get("/")]
async fn hello() -> &'static str {
    "Hello, Miko!"
}

// Use #[miko] macro to automatically configure and run the application (Recommended)
#[miko]
async fn main() {
    // router is automatically created
    // routes are automatically collected and registered
    // config is automatically loaded
}
```

> **What `#[miko]` macro does**:
> - Expands to `#[tokio::main]`
> - Creates `router: Router` automatically
> - Collects routes defined by `#[get]`, `#[post]` macros and registers them (requires `auto` feature)
> - Loads `config.toml` and merges `config.{dev/prod}.toml`
> - Initializes the global dependency container (requires `auto` feature)
> - Runs the application

Run the application:

```bash
car go run
```

Visit `http://localhost:8080`, and you will see "Hello, Miko!".

### Without Macro

If you prefer not to use the `#[miko]` macro, you can configure it manually:

```rust
use miko::*
use miko::macros::*

#[tokio::main]
async fn main() {
    let router = Router::new()
        .get("/", || async { "Hello, Miko!" });

    // Use default config without loading file
    let config = ApplicationConfig::default();
    Application::new(config, router).run().await.unwrap();
}
```

## JSON API

Now let's create an API that returns JSON:

```rust
use miko::{*, extractor::Json};
use miko::macros::*
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Message {
    text: String,
    timestamp: u64,
}

#[get("/api/message")]
async fn get_message() -> Json<Message> {
    Json(Message {
        text: "Hello from Miko!".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

#[miko]
async fn main() {
    // Routes auto-registered
}
```

Visit `http://localhost:8080/api/message` to get the JSON response.

## Handling Request Data

### Path Parameters

```rust
use miko::{*, extractor::{Json, Path}};
use miko::macros::*

#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
}

#[get("/users/{id}")]
async fn get_user(Path(id): Path<u32>) -> Json<User> {
    Json(User {
        id,
        name: format!("User {}", id),
    })
}
```

Visit `http://localhost:8080/users/123`

### Query Parameters

```rust
use std::collections::HashMap;
use miko::{*, extractor::Query};
use miko::macros::*

#[derive(Deserialize)]
struct QueryStruct {
    q: String
}

#[get("/search")]
async fn search(Query(params): Query<QueryStruct>) -> String {
    let keyword = params.q;
    format!("Searching for: {}", keyword)
}
```

Visit `http://localhost:8080/search?q=rust`

### POST Request Body

```rust
#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: u32,
    name: String,
    email: String,
}

#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> Json<UserResponse> {
    Json(UserResponse {
        id: 1,
        name: data.name,
        email: data.email,
    })
}
```

Test:

```bash
curl -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","email":"alice@example.com"}'
```

## Complete Example

A complete application with multiple routes:

```rust
use miko::{*, extractor::{Json, Path, Query}};
use miko::macros::*
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

// Home
#[get("/")]
async fn index() -> &'static str {
    "Welcome to Miko API!"
}

#[derive(serde::Deserialize)]
struct ListQuery {
    limit: Option<usize>,
}

#[get("/users")]
async fn list_users(Query(params): Query<ListQuery>) -> Json<Vec<User>> {
    let limit = params.limit.unwrap_or(10) as u32;
    let users: Vec<User> = (1..=limit)
        .map(|id| User {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        })
        .collect();
    Json(users)
}

// Get single user
#[get("/users/{id}")]
async fn get_user(Path(id): Path<u32>) -> Json<User> {
    Json(User {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
    })
}

// Create user
#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> Json<User> {
    Json(User {
        id: 999,
        name: data.name,
        email: data.email,
    })
}

#[miko]
async fn main() {
    // All routes auto-registered
}
```

## Testing API

```bash
# Visit home
curl http://localhost:8080/

# Get user list
curl http://localhost:8080/users

# Get user list (limit count)
curl http://localhost:8080/users?limit=5

# Get single user
curl http://localhost:8080/users/1

# Create user
curl -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","email":"alice@example.com"}'
```

## Configuring Server

By default, Miko listens on `0.0.0.0:8080`. You can change this via a configuration file:

Create `config.toml`:

```toml
[application]
addr = "127.0.0.1"
port = 9999
```

The `#[miko]` macro automatically loads the configuration. In development environment (debug mode), if `config.dev.toml`
exists, it will merge with `config.toml`; in production environment (release mode), `config.prod.toml` will be used.

For more configuration options, see [Configuration Management](configuration_management.md).

## Next Steps

Congratulations! You have mastered the basics of Miko. Next you can:

- 📖 Read [Basic Concepts](basic_concepts.md) to dive deeper into the architecture.
- 🛣️ Learn advanced [Routing System](routing_system.md).
- 🔍 Explore [Error Handling](error_handling.md) mechanism.
- 💉 Use [Dependency Injection](dependency_injection.md) to manage components.
- 📝 Integrate [OpenAPI](openapi_integration.md) to generate docs.
- ✅ Add [Data Validation](data_validation.md).

Check the `miko/examples/` directory for more complete examples.
