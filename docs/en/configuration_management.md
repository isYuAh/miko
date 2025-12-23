# Configuration Management

In v0.8.0, Miko introduced a brand new configuration system based on the robust `config` crate, supporting multi-format
configuration files (TOML, YAML, JSON, JSON5), environment variable injection, and default values.

## Configuration Files

### Supported Formats

Miko enables TOML support by default. You can enable other formats via features:

```toml
[dependencies]
# Default support for TOML
miko = "0.8"

# Enable YAML support
miko = { version = "0.8", features = ["config-yaml"] }
```

### Basic Configuration

Create `config.toml` (or `config.yaml`, `config.json`, etc.) in the project root:

```toml
# [server] replaces the old [application] section
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgres://localhost/mydb"
max_connections = 10

[app]
name = "My Application"
debug = false
```

### Environment Configuration

Miko automatically loads additional configuration files based on the runtime environment, overriding the basic
configuration:

- **Development** (`debug` build or `CONFIG_ENV=dev`): Loads `config.dev.{ext}`
- **Production** (`release` build or `CONFIG_ENV=prod`): Loads `config.prod.{ext}`

For example, `config.dev.toml`:

```toml
[server]
port = 3000

[app]
debug = true
```

### Environment Variables

Miko supports overriding configuration via environment variables. Variables must start with `MIKO__`, using double
underscores `__` as separators.

Examples:

- `MIKO__SERVER__PORT=9090` overrides `server.port`
- `MIKO__DATABASE__URL=...` overrides `database.url`

## Automatic Loading

When using the `#[miko]` macro, the configuration system is automatically initialized:

```rust
use miko::*;
use miko::macros::*;

#[miko]
async fn main() {
    // 1. Load config.{toml,yaml,json}
    // 2. Load config.{env}.{toml,yaml,json}
    // 3. Load MIKO__ environment variables
    // 4. Apply server settings and start
}
```

## Using Configuration in Handlers

### Using `#[config]` Annotation

You can inject configuration values directly into Handler parameters.

#### 1. Basic Usage

```rust
#[get("/info")]
async fn app_info(
    #[config("app.name")] app_name: String,
    #[config("server.port")] port: u16,
) -> String {
    format!("App: {}, Port: {}", app_name, port)
}
```

#### 2. Injection with Default Values (New in v0.8)

If a configuration item might not exist, you can provide a default value (literals supported):

```rust
#[get("/feature")]
async fn feature_flag(
    // If "app.enable_beta" is missing, use default false
    #[config("app.enable_beta:false")] beta: bool,

    // String default
    #[config("app.theme:dark")] theme: String,

    // Numeric default
    #[config("app.max_items:100")] max_items: u32,
) -> String {
    format!("Beta: {}, Theme: {}, Max: {}", beta, theme, max_items)
}
```

#### 3. Injecting Structs

`#[config]` supports injecting any struct that implements `Deserialize`:

```rust
#[derive(Deserialize)]
struct DatabaseConfig {
    url: String,
    max_connections: u32,
}

#[get("/db")]
async fn db_info(
    #[config("database")] db: DatabaseConfig
) -> String {
    format!("DB URL: {}", db.url)
}
```

## Programmatic Access

If you need to access configuration outside of Handlers (e.g., in `main` or custom components):

```rust
use miko::app::config::{get_settings, get_settings_value};

fn some_function() {
    // 1. Get the entire configuration object (config::Config)
    let settings = get_settings();
    let port = settings.get_int("server.port").unwrap_or(8080);

    // 2. Get specific typed configuration value (supports generics)
    let app_name: String = get_settings_value("app.name").unwrap_or_default();
}
```

## Migration Guide (v0.7 -> v0.8)

If you are upgrading from an older version, please note the following breaking changes:

1. **Config Section Change**: `[application]` section is renamed to `[server]`.
    * `addr` -> `host`
    * `port` -> `port`
2. **API Changes**:
    * `ApplicationConfig` is removed, use `ServerSettings`.
    * `ApplicationConfig::load_()` is removed.
3. **Dependency Change**: Replaced `toml` crate with `config` crate.

## Next Steps

- 💉 Use [Dependency Injection](dependency_injection.md) to manage components
- 🔍 Understand [Request Extractors](request_extractors.md) usage
