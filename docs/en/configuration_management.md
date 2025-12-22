# Configuration Management

Miko provides a TOML-based configuration system, supporting environment separation and configuration injection.

## Configuration Files

### Basic Configuration

Create `config.toml` in the project root:

```toml
[application]
addr = "0.0.0.0"
port = 8080

[database]
url = "postgres://localhost/mydb"
max_connections = 10

[redis]
url = "redis://localhost"
timeout = 5

[app]
name = "My Application"
version = "1.0.0"
debug = false
```

### Environment-specific Configuration

Miko supports environment-specific configuration files:

- `config.dev.toml` - Development environment (debug mode)
- `config.prod.toml` - Production environment (release mode)

Environment configuration will be automatically merged with the basic configuration, with environment settings having
higher priority.

**config.dev.toml**:

```toml
[application]
port = 3000

[app]
debug = true
```

**config.prod.toml**:

```toml
[database]
url = "postgres://prod-server/mydb"
max_connections = 50

[app]
debug = false
```

## Automatic Loading

When using the `#[miko]` macro, configuration is automatically loaded:

```rust
use miko::*;
use miko::macros::*;

#[get("/")]
async fn index() -> &'static str {
    "Hello"
}

#[miko]
async fn main() {
    // Configuration automatically loaded
    // router is available
}
```

The `#[miko]` macro will:

1. Load `config.toml`.
2. Merge `config.dev.toml` or `config.prod.toml` based on the compilation mode.
3. Store the configuration in global variables.

## Manual Loading

If you don't use the `#[miko]` macro, there are two ways to manually load configuration:

### Method 1: Using `Application::new_` (Recommended)

`Application::new_` automatically loads configuration files:

```rust
use miko::{Router, Application};

#[tokio::main]
async fn main() {
    let router = Router::new()
        .get("/", handler);

    // Automatically loads config.toml and environment-specific configs
    Application::new_(router).run().await.unwrap();
}
```

### Method 2: Manual Loading

If you need to access configuration before creating the application:

```rust
use miko::app::config::ApplicationConfig;
use miko::{Router, Application};

#[tokio::main]
async fn main() {
    // Manually load config
    let config = ApplicationConfig::load_().unwrap_or_default();

    println!("Starting on port {}", config.port);

    let router = Router::new()
        .get("/", handler);

    Application::new(config, router).run().await.unwrap();
}
```

## Using Configuration in Handlers

### Using `#[config]` Annotation

Inject configuration values directly into Handler parameters:

```rust
#[get("/info")]
async fn app_info(
    #[config("app.name")] app_name: String,
    #[config("app.version")] version: String,
    #[config("app.debug")] debug: bool,
) -> Json<serde_json::Value> {
    Json(json!({
        "name": app_name,
        "version": version,
        "debug": debug
    }))
}
```

### Configuration Paths

Access nested configuration using dot-separated paths:

```toml
[database]
host = "localhost"
port = 5432

[database.pool]
min = 5
max = 20
```

```rust
#[get("/db-config")]
async fn db_config(
    #[config("database.host")] host: String,
    #[config("database.port")] port: u16,
    #[config("database.pool.max")] max_connections: u32,
) -> String {
    format!("DB: {}:{}, Max: {}", host, port, max_connections)
}
```

### Supported Types

`#[config]` supports all types that implement `serde::Deserialize`:

```rust
// Basic types
#[config("port")] port: u16
#[config("debug")] debug: bool
#[config("name")] name: String
#[config("timeout")] timeout: f64

// Collection types
#[config("allowed_origins")] origins: Vec<String>
#[config("features")] features: HashMap<String, bool>

// Custom structs
#[derive(Deserialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    database: String,
}

#[get("/db-config")]
async fn db_config(
    #[config("database")] db_config: DatabaseConfig
) -> String {
    format!("{}:{}/{}", db_config.host, db_config.port, db_config.database)
}

// Optional types
#[get("/optional")]
async fn optional_config(
    #[config("features.beta")] beta: Option<bool>
) -> String {
    format!("Beta enabled: {}", beta.unwrap_or(false))
}
```

## Programmatic Access to Configuration

### Get Configuration Section

```rust
use miko::app::config::load_config_section;
use serde::Deserialize;

#[derive(Deserialize)]
struct DatabaseConfig {
    url: String,
    max_connections: u32,
}

async fn init_db() -> Result<Database, Error> {
    let config: DatabaseConfig = load_config_section("database")?;
    Database::connect(&config.url).await
}
```

### Get Full Configuration

```rust
use miko::app::config::get_config;
use toml::Value;

fn get_feature_flag(name: &str) -> bool {
    let config = get_config();
    config.get("features")
        .and_then(|f| f.get(name))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}
```

## Next Steps

- 💉 Use [Dependency Injection](dependency_injection.md) to manage components.
- 🔍 Understand [Request Extractor](request_extractors.md) usage.
- 📖 Review [Basic Concepts](basic_concepts.md) to understand the architecture.
