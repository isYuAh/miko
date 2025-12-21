# Miko

<div align="center">

**一个现代化、高性能的 Rust Web 框架**

[![Crates.io](https://img.shields.io/crates/v/miko.svg)](https://crates.io/crates/miko)
[![Documentation](https://docs.rs/miko/badge.svg)](https://docs.rs/miko)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[中文](README.md) | [English](README.en.md)

</div>

## ✨ 特性

- 🚀 **高性能** - 基于 Hyper 和 Tokio 构建，充分利用 Rust 的异步特性
- 🎯 **类型安全** - 完整的类型推导，编译时捕获错误
- 🔌 **模块化设计** - 通过 features 按需启用功能
- 🎨 **优雅的宏** - 提供简洁直观的路由定义宏
- 🔄 **依赖注入** - 内置依赖容器，支持组件自动装配
- 📝 **OpenAPI 支持** - 无缝集成 utoipa，自动生成 API 文档
- ✅ **数据验证** - 集成 garde，提供强大的数据验证能力
- 🌐 **WebSocket** - 原生 WebSocket 支持
- 🔍 **统一错误处理** - 优雅的错误处理机制
- 🔄 **优雅停机** - 支持信号监听与连接平滑关闭
- 🎭 **Tower 生态** - 兼容 Tower 中间件生态

## 🚀 快速开始

### 安装

```bash
cargo add miko --features=full
```

### Hello World

```rust
use miko::*;
use miko::macros::*;

#[get("/")]
async fn hello() -> &'static str {
    "Hello, Miko!"
}

#[main]
async fn main() {
}
```

运行程序后访问 `http://localhost:8080`

### 更多示例

```rust
use miko::{*, macros::*, extractor::{Json, Path, Query}};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

// 使用路由宏和提取器
#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> Json<User> {
    Json(User {
        id: 1,
        name: data.name,
        email: data.email,
    })
}

// 路径参数
#[get("/users/{id}")]
async fn get_user(Path(id): Path<u32>) -> Json<User> {
    Json(User {
        id,
        name: "Alice".into(),
        email: "alice@example.com".into(),
    })
}

```rust
// 查询参数
#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[get("/search")]
async fn search(Query(params): Query<SearchQuery>) -> String {
    format!("Searching for: {:?}", params)
}
```

```rust
#[tokio::main]
async fn main() {
    let router = Router::new()
        .post("/users", create_user)
        .get("/users/{id}", get_user)
        .get("/search", search);

    Application::new_(router).run().await.unwrap();
}
```

## 📚 文档

- **[快速上手](docs/zh/快速上手.md)** - 5分钟入门教程
- **[基础概念](docs/zh/基础概念.md)** - 核心概念详解
- **[路由系统](docs/zh/路由系统.md)** - 路由定义与管理
- **[请求提取器](docs/zh/请求提取器.md)** - 提取请求数据
- **[响应处理](docs/zh/响应处理.md)** - 构建各种响应
- **[错误处理](docs/zh/错误处理.md)** - 统一错误处理
- **[中间件与层](docs/zh/中间件与层.md)** - 中间件使用
- **[依赖注入](docs/zh/依赖注入.md)** - 组件管理
- **[WebSocket 支持](docs/zh/WebSocket支持.md)** - WebSocket 开发
- **[配置管理](docs/zh/配置管理.md)** - 应用配置
- **[OpenAPI 集成](docs/zh/OpenAPI集成.md)** - API 文档生成
- **[数据验证](docs/zh/数据验证.md)** - 请求数据验证
- **[高级特性](docs/zh/高级特性.md)** - 进阶功能

## 🎯 Features

Miko 采用模块化设计，你可以按需启用功能：

```toml
[dependencies]
# 默认启用核心功能（宏、自动注册、扩展功能）
miko = "x.x"

# 或启用所有功能，包括 OpenAPI 和数据验证
miko = { version = "x.x", features = ["full"] }

# 或只启用需要的功能
miko = { version = "x.x", features = ["utoipa", "validation"] }
```

可用的 features：

- `default` - 核心功能（`macro` + `auto` + `ext`），**默认启用**
- `full` - 启用所有功能（包括外部扩展）
- `macro` - 启用路由宏（`#[get]`、`#[post]` 等）
- `auto` - 启用自动路由注册和依赖注入
- `ext` - 启用扩展功能（快速CORS、静态文件等）
- `utoipa` - 启用 OpenAPI 文档生成（自动重导出 `utoipa` crate）
- `validation` - 启用数据验证（自动重导出 `garde` crate）

**注意**：当启用 `utoipa` 或 `validation` feature 时，无需在你的 `Cargo.toml` 中手动添加这些依赖，框架会自动重导出它们：

```rust
// 启用 utoipa feature 后，直接使用
use miko::{utoipa, OpenApi, ToSchema};

// 启用 validation feature 后，直接使用
use miko::{garde, Validate};
```

## 🛠️ 核心组件

### 路由宏

使用简洁的宏定义路由：

```rust
#[get("/users")]
async fn list_users() -> Json<Vec<User>> { /* ... */ }

#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> AppResult<Json<User>> { /* ... */ }

#[put("/users/{id}")]
async fn update_user(Path(id): Path<u32>, Json(data): Json<UpdateUser>) -> AppResult<Json<User>> { /* ... */ }

#[delete("/users/{id}")]
async fn delete_user(Path(id): Path<u32>) -> AppResult<()> { /* ... */ }
```

### 依赖注入

使用 `#[component]` 和 `#[dep]` 实现依赖注入：

```rust
#[component]
impl Database {
    async fn new() -> Self {
        // 初始化数据库连接
        Self { /* ... */ }
    }
}

#[get("/users")]
async fn list_users(#[dep] db: Arc<Database>) -> Json<Vec<User>> {
    // 使用注入的数据库实例
    Json(vec![])
}
```

默认情况下组件以单例模式注册；可通过 `#[component(transient)]` 或 `#[component(mode = "transient")]` 让每次注入都创建新的实例（此模式下不支持 `prewarm`）。

### OpenAPI 文档

自动生成 API 文档：（只是推断params summary descripion required这些，其他的还需要自己写，比如opanapi的一个结构体，还有paths这种）

```rust
use miko::*;

#[derive(Serialize, Deserialize, ToSchema)]
struct User {
    id: u32,
    name: String,
}

#[get("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功", body = User)]
async fn get_user(
    #[path] #[desc("用户ID")] id: u32
) -> Json<User> {
    // ...
}
```

### 数据验证

使用 `ValidatedJson` 自动验证：

```rust
use garde::Validate;

#[derive(Deserialize, Validate)]
struct CreateUser {
    #[garde(length(min = 3, max = 50))]
    name: String,

    #[garde(contains("@"))]
    email: String,
}

#[post("/users")]
async fn create_user(
    ValidatedJson(data): ValidatedJson<CreateUser>
) -> Json<User> {
    // 数据已通过验证
}
```

## 🌟 示例

`miko/examples/` 目录中提供了一个功能全面的 `all-in-one` 示例：

- **[basic.rs](./miko/examples/basic.rs)**

该示例覆盖了框架的绝大多数核心功能，包括路由、中间件、依赖注入、WebSocket、文件上传等。强烈建议通过此文件来快速了解 Miko 的用法。

运行该示例：

```bash
cargo run --example basic --features full
```

## 🤝 贡献

我们欢迎任何形式的贡献。有关如何贡献代码的详细信息，请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 📄 许可证

## 🔗 相关链接

- [GitHub 仓库](https://github.com/isyuah/miko)
- [crates.io](https://crates.io/crates/miko)
- [文档](https://docs.rs/miko)

## 💬 社区与支持

- 提交 Issue: [GitHub Issues](https://github.com/isyuah/miko/issues)
- 讨论: [GitHub Discussions](https://github.com/isyuah/miko/discussions)
