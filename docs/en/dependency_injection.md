# Dependency Injection

> **Requires `auto` feature**

Miko provides a built-in dependency injection (DI) container, enabling automatic component wiring and injection through
the `#[component]` and `#[dep]` macros.

## Basic Concepts

Dependency Injection (DI) allows you to:

- Decouple component creation from its usage.
- Automatically obtain shared instances in Handlers.
- Simplify testing and modular design.

## Defining Components

Use the `#[component]` macro to mark a type as a component:

```rust
use miko::*;
use miko::macros::*;
use std::sync::Arc;

#[component]
impl Database {
    async fn new() -> Self {
        // Initialize database connection
        println!("Initializing database connection...");
        Self {
            pool: create_connection_pool().await,
        }
    }

    pub fn query_users(&self) -> Vec<User> {
        // Query logic
        vec
        []
    }
}
```

### Component Requirements

- Must implement `async fn new() -> Self`.
- The `new` method is used to create the component instance.
- Components are shared by being wrapped in `Arc<T>`.

## Injecting Components

Use `#[dep]` in Handlers to inject components:

```rust
#[get("/users")]
async fn list_users(#[dep] db: Arc<Database>) -> Json<Vec<User>> {
    let users = db.query_users();
    Json(users)
}

#[get("/users/{id}")]
async fn get_user(
    #[path] id: u32,
    #[dep] db: Arc<Database>,
) -> AppResult<Json<User>> {
    let user = db.find_user(id)
        .ok_or(AppError::NotFound("User not found".into()))?;
    Ok(Json(user))
}
```

### Multiple Dependencies

A single Handler can inject multiple components:

```rust
#[component]
impl Cache {
    async fn new() -> Self {
        Self { redis: connect_redis().await }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        // ...
    }
}

#[get("/users/{id}")]
async fn get_user(
    #[path] id: u32,
    #[dep] db: Arc<Database>,
    #[dep] cache: Arc<Cache>,
) -> AppResult<Json<User>> {
    // Check cache first
    if let Some(cached) = cache.get(&format!("user:{}", id)) {
        return Ok(Json(serde_json::from_str(&cached)?));
    }

    // Query database
    let user = db.find_user(id)?;
    Ok(Json(user))
}
```

## Complete Example

```rust
use miko::*;
use miko::macros::*;
use std::sync::Arc;

// Database Component
#[component]
impl Database {
    async fn new() -> Self {
        println!("📦 Initializing database...");
        Self {
            // Connection pool creation would happen here
        }
    }

    pub fn get_user(&self, id: u32) -> Option<User> {
        // Query logic
        Some(User {
            id,
            name: "Alice".into(),
            email: "alice@example.com".into(),
        })
    }
}

// Cache Component
#[component]
impl Cache {
    async fn new() -> Self {
        println!("📦 Initializing cache...");
        Self {
            // Redis connection etc.
        }
    }

    pub fn set(&self, key: &str, value: &str) {
        println!("Cache SET: {} = {}", key, value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        None  // Example
    }
}

// Logger Service Component
#[component]
impl Logger {
    async fn new() -> Self {
        println!("📦 Initializing logger...");
        Self {}
    }

    pub fn log(&self, message: &str) {
        println!("[LOG] {}", message);
    }
}

#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

// Using Dependency Injection
#[get("/users/{id}")]
async fn get_user(
    #[path] id: u32,
    #[dep] db: Arc<Database>,
    #[dep] cache: Arc<Cache>,
    #[dep] logger: Arc<Logger>,
) -> AppResult<Json<User>> {
    logger.log(&format!("Fetching user {}", id));

    // Try to get from cache
    let cache_key = format!("user:{}", id);
    if let Some(cached) = cache.get(&cache_key) {
        logger.log("Cache hit!");
        return Ok(Json(serde_json::from_str(&cached)?));
    }

    // Query from database
    let user = db.get_user(id)
        .ok_or(AppError::NotFound("User not found".into()))?;

    // Write to cache
    cache.set(&cache_key, &serde_json::to_string(&user)?);

    Ok(Json(user))
}

#[miko]
async fn main() {
    // Components are automatically registered and initialized
    println!("🚀 Server running");
}
```

## Dependency Container

### Automatic Initialization

When using the `#[miko]` macro, the dependency container is initialized automatically:

```rust
#[miko]
async fn main() {
    // 1. Dependency container initialization
    // 2. Route collection
    // 3. Application start
}
```

### Prewarming

By default, components are lazily created (on first use). You can enable prewarming:

```rust
#[component(prewarm = true)]
impl Database {
    async fn new() -> Self {
        // Initialize immediately when the application starts
        Self { /* ... */ }
    }
}
```

Prewarmed components are initialized asynchronously after the application starts, without blocking the server startup.

## Component Lifecycles

### Singleton Mode (Default)

By default, a component is created only once during the application's lifecycle and shared as an `Arc<T>` across all
Handlers:

```rust
#[component]
impl Database {
    async fn new() -> Self {
        println!("This will only print once!");
        Self {}
    }
}
```

> Tip: `#[component(prewarm)]` only works for singleton mode; it constructs the instance ahead of time after application
> startup.

### Transient Mode

If you want a brand-new instance for every injection, use the `transient` mode:

```rust
#[component(transient)]
impl RequestScopedLogger {
    async fn new() -> Self {
        println!("This runs on every injection");
        Self::default()
    }
}
```

You can also explicitly write `#[component(mode = "singleton")]` / `#[component(mode = "transient")]` for clearer
intent. Transient components execute the `new` method on every Handler call and thus do not support `prewarm`.

### Shared References

Components are shared via `Arc<T>`, making them safe to use across multiple Handlers and threads:

```rust
#[get("/route1")]
async fn handler1(#[dep] db: Arc<Database>) {
    // Uses the same Database instance
}

#[get("/route2")]
async fn handler2(#[dep] db: Arc<Database>) {
    // Uses the same Database instance
}
```

## Inter-Component Dependencies

### Constructor Injection

A component's `new` function can receive other components as dependencies (must be of type `Arc<T>`):

```rust
// Base Components
#[component]
impl Database {
    async fn new() -> Self {
        Self { pool: create_pool().await }
    }
}

#[component]
impl Cache {
    async fn new() -> Self {
        Self { redis: connect_redis().await }
    }
}

// Component depending on other components
#[component]
impl UserService {
    async fn new(
        db: Arc<Database>,      // Inject Database
        cache: Arc<Cache>,      // Inject Cache
    ) -> Self {
        println!("UserService initialized with db and cache");
        Self { db, cache }
    }

    pub fn get_user(&self, id: u32) -> Option<User> {
        // Can directly use injected dependencies
        if let Some(cached) = self.cache.get(&format!("user:{}", id)) {
            return Some(cached);
        }
        self.db.find_user(id)
    }
}
```

Using the combined service:

```rust
#[get("/users/{id}")]
async fn get_user(
    #[path] id: u32,
    #[dep] user_service: Arc<UserService>,  // Directly inject the combined service
) -> AppResult<Json<User>> {
    let user = user_service.get_user(id)
        .ok_or(AppError::NotFound("User not found".into()))?;
    Ok(Json(user))
}
```

### Dependency Resolution Order

The framework automatically analyzes dependencies and initializes components in the correct order:

```rust
// Initialization Order: Database -> Cache -> UserService
#[component]
impl Database {
    async fn new() -> Self { /* ... */ }
}

#[component]
impl Cache {
    async fn new() -> Self { /* ... */ }
}

#[component]
impl UserService {
    async fn new(db: Arc<Database>, cache: Arc<Cache>) -> Self { /* ... */ }
}
```

> **Note**: Do not create circular dependencies (A depends on B, B depends on A), as this will cause initialization
> failure.

## Difference from State

| Feature           | Dependency Injection `#[dep]` | Global State `State<T>`  |
|-------------------|-------------------------------|--------------------------|
| Definition        | `#[component]` macro          | `Router::with_state()`   |
| Auto-registration | Yes                           | No                       |
| Multiple Types    | Multiple different types      | Single type              |
| Use Case          | Multiple independent services | Shared application state |
| Feature Required  | Requires `auto`               | No feature required      |

### When to Use Dependency Injection

- ✅ Multiple independent services (database, cache, logging, etc.).
- ✅ Automatic initialization required.
- ✅ Modular code preferred.
- **When using `#[miko]`, manual state setting is likely impossible due to automatic route registration.**

### When to Use State

- ✅ Simple shared state.
- ✅ `auto` feature not required.
- ✅ Only one state object.
- **Or when you haven't enabled `auto` and cannot use DI.**

## Real-world Application Architecture Example

```rust
use miko::*;
use miko::macros::*;
use std::sync::Arc;

// Data Access Layer
#[component]
impl UserRepository {
    async fn new() -> Self {
        Self {
            pool: create_db_pool().await,
        }
    }

    pub async fn find_by_id(&self, id: u32) -> Option<User> {
        // DB query
    }

    pub async fn create(&self, data: CreateUser) -> Result<User, Error> {
        // Data insertion
    }
}

// Authentication Service
#[component]
impl AuthService {
    async fn new() -> Self {
        Self {
            jwt_secret: std::env::var("JWT_SECRET").unwrap(),
        }
    }

    pub fn verify_token(&self, token: &str) -> Option<UserId> {
        // Verify JWT
    }

    pub fn generate_token(&self, user_id: u32) -> String {
        // Generate JWT
    }
}

// Email Service
#[component]
impl EmailService {
    async fn new() -> Self {
        Self {
            smtp_config: load_smtp_config(),
        }
    }

    pub async fn send(&self, to: &str, subject: &str, body: &str) {
        // Send email
    }
}

// Handler using multiple services
#[post("/register")]
async fn register(
    Json(data): Json<RegisterData>,
    #[dep] users: Arc<UserRepository>,
    #[dep] auth: Arc<AuthService>,
    #[dep] email: Arc<EmailService>,
) -> AppResult<Json<AuthResponse>> {
    // Create user
    let user = users.create(data.into()).await?;

    // Generate token
    let token = auth.generate_token(user.id);

    // Send welcome email
    email.send(
        &user.email,
        "Welcome!",
        "Thanks for registering"
    ).await;

    Ok(Json(AuthResponse { token, user }))
}

#[miko]
async fn main() {
    println!("🚀 Server starting...");
}
```

## Next Steps

- 🔧 Learn [Configuration Management](configuration_management.md) for injecting config.
- 🔐 Use [Middleware](middleware_and_layers.md) to add global features.
- 📖 Review [Basic Concepts](basic_concepts.md) to understand the architecture.
