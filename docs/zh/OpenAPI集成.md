# OpenAPI 集成

> **需要 `utoipa` feature**
> 由于扫描并生成utoipa path是通过get等宏实现的，所以写的时候务必将#[get]放在最顶端

Miko 集成 [utoipa](https://github.com/juhaku/utoipa) 库,为路由自动生成 OpenAPI 3.0 文档。

## Miko 提供的功能

### 1. 自动推断 OpenAPI 信息

Miko 的路由宏 (`#[get]`, `#[post]` 等) 会自动分析 handler 函数,推断并生成以下 OpenAPI 信息:

- **路径参数**: 从 `#[path]` 标注自动识别参数名称和类型
- **查询参数**: 从 `#[query]` 标注自动识别查询参数结构
- **请求体**: 从 `Json<T>` 等提取器自动识别请求体类型
- **文档注释**: 自动提取 `///` 注释作为 API 描述(首行→摘要,其余行→详细描述)

⚠️ **注意**: Miko **不会自动推断响应体**,因为返回类型是 `impl IntoResponse`,无法确定具体响应模型,需要使用 `#[u_response]` 显式标注。

```rust
/// 获取用户信息
///
/// 根据用户 ID 查询用户详细信息
#[get("/users/{id}")]
async fn get_user(
    #[path] id: u32,           // ✅ 自动生成: 参数名 "id", 类型 integer
    #[query] filter: Filter,   // ✅ 自动生成: query 参数结构
    Json(data): Json<User>,    // ✅ 自动生成: 请求体 application/json
) -> Json<User> {
    // ✅ 自动提取文档注释: summary = "获取用户信息", description = "根据用户 ID..."
    // ❌ 但响应体需要手动标注(见下方示例)
}
```

**自动提取文档注释的规则**:
- 第一行 `///` 注释 → OpenAPI `summary`
- 其余 `///` 注释行 → OpenAPI `description`
- 可以用 `#[u_summary]` 和 `#[u_description]` 宏覆盖自动提取的内容

### 2. 文档注解宏

Miko 提供了一系列宏来补充 OpenAPI 文档信息:

| 宏 | 用途 | 示例 |
|---|-----|------|
| `#[u_tag]` | 设置 API 标签分组 | `#[u_tag("用户管理")]` |
| `#[u_response]` | 定义响应状态和模型 | `#[u_response(status = 200, body = User)]` |
| `#[u_summary]` | 设置 API 摘要 | `#[u_summary("获取用户信息")]` |
| `#[u_description]` | 设置详细描述 | `#[u_description("根据 ID 查询用户")]` |
| `#[u_request_body]` | 自定义请求体类型 | `#[u_request_body(content = Multipart)]` |
| `#[u_param]` | 补充参数信息 | `#[u_param(name = "id", example = 123)]` |
| `#[u_deprecated]` | 标记 API 已废弃 | `#[u_deprecated]` |
| `#[desc]` | 为参数添加描述 | `#[path] #[desc("用户ID")] id: u32` |

## 快速开始

### 1. 添加依赖

```toml
[dependencies]
miko = { version = "0.3.5", features = ["full"] }

[dev-dependencies]
utoipa-scalar = { version = "0.2", features = ["axum"] }
```

### 2. 定义 Schema

为数据结构派生 `ToSchema`：

```rust
use miko::*;
use miko::macros::*;

#[derive(Serialize, Deserialize, ToSchema)]
struct User {
    #[schema(example = 1)]
    id: u32,

    #[schema(example = "Alice")]
    name: String,

    #[schema(example = "alice@example.com")]
    email: String,
}
```

### 3. 添加路由文档

**必须使用 `#[u_response]` 标注响应体**:

```rust
/// 获取用户信息
///
/// 根据用户 ID 查询并返回用户详细信息
#[get("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功", body = User)]  // ← 必须显式标注响应体
#[u_response(status = 404, description = "用户不存在")]
async fn get_user(
    #[path] #[desc("用户ID")] id: u32
) -> AppResult<Json<User>> {
    // Miko 自动推断: 路径参数 id
    // Miko 不推断: 响应体(需要 #[u_response] 标注)
}
```

**可选: 用宏覆盖自动提取的文档注释**:

```rust
/// 这个注释会被下面的宏覆盖
#[get("/users/{id}")]
#[u_summary("查询用户")]  // ← 覆盖文档注释的第一行
#[u_description("通过 ID 获取用户信息")]  // ← 覆盖文档注释的其余行
#[u_response(status = 200, body = User)]
async fn get_user(#[path] id: u32) -> Json<User> {
    // 最终 OpenAPI: summary = "查询用户", description = "通过 ID 获取用户信息"
}
```

### 4. 生成 OpenAPI 文档

```rust
use miko::OpenApi;
use miko::macros::*;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Blog API",
        version = "1.0.0",
        description = "A simple blog API"
    ),
    servers(
        (url = "http://localhost:8080", description = "Local server")
    ),
    tags(
        (name = "用户管理", description = "用户相关接口"),
        (name = "文章管理", description = "文章相关接口")
    )
)]
struct ApiDoc;

#[route("/openapi.json", method = "get")]
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
```

### 5. 集成 Scalar UI

```rust
use utoipa_scalar::{Scalar, Servable};

#[route("/scalar", method = "get")]
async fn scalar_ui() -> impl IntoResponse {
    Scalar::new("/openapi.json").into_response()
}

#[miko]
async fn main() {
    println!("📚 Scalar UI: http://localhost:8080/scalar");
    println!("📄 OpenAPI JSON: http://localhost:8080/openapi.json");
}
```

## 文档注解

### 基础注解

```rust
/// API 端点描述
///
/// 更详细的说明可以写在这里，支持 Markdown 格式
#[get("/users")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功", body = Vec<User>)]
async fn list_users() -> Json<Vec<User>> {
    // ...
}
```

### 参数文档

使用 `#[desc]` 为参数添加描述：

```rust
#[get("/users/{id}")]
async fn get_user(
    #[path] #[desc("用户的唯一标识符")] id: u32,
    #[query] #[desc("是否包含详细信息")] include_details: Option<bool>,
) -> AppResult<Json<User>> {
    // ...
}
```

### 响应文档

定义多个可能的响应状态码：

```rust
#[post("/users")]
#[u_tag("用户管理")]
#[u_response(status = 201, description = "创建成功", body = User)]
#[u_response(status = 400, description = "请求参数错误")]
#[u_response(status = 409, description = "用户已存在")]
async fn create_user(
    Json(data): Json<CreateUser>
) -> AppResult<(StatusCode, Json<User>)> {
    // ...
}
```

## 完整示例

```rust
use miko::*;
use miko::macros::*;
use utoipa_scalar::{Scalar, Servable};

// ========== Schemas ==========

#[derive(Serialize, Deserialize, ToSchema)]
struct User {
    #[schema(example = 1)]
    id: u32,

    #[schema(example = "Alice")]
    name: String,

    #[schema(example = "alice@example.com")]
    email: String,
}

#[derive(Deserialize, ToSchema)]
struct CreateUser {
    #[schema(example = "Bob", min_length = 3)]
    name: String,

    #[schema(example = "bob@example.com")]
    email: String,
}

#[derive(Serialize, ToSchema)]
struct ErrorResponse {
    error: String,
    message: String,
}

// ========== Handlers ==========

/// 获取所有用户
///
/// 返回系统中所有用户的列表
#[get("/users")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功返回用户列表", body = Vec<User>)]
async fn list_users() -> Json<Vec<User>> {
    Json(vec![
        User {
            id: 1,
            name: "Alice".into(),
            email: "alice@example.com".into(),
        }
    ])
}

/// 获取单个用户
///
/// 根据用户 ID 查询用户信息
#[get("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功", body = User)]
#[u_response(status = 404, description = "用户不存在", body = ErrorResponse)]
async fn get_user(
    #[path] #[desc("用户ID")] id: u32
) -> AppResult<Json<User>> {
    Ok(Json(User {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
    }))
}

/// 创建用户
///
/// 创建一个新用户
#[post("/users")]
#[u_tag("用户管理")]
#[u_response(status = 201, description = "创建成功", body = User)]
#[u_response(status = 400, description = "请求参数错误", body = ErrorResponse)]
#[u_response(status = 409, description = "用户已存在", body = ErrorResponse)]
async fn create_user(
    Json(data): Json<CreateUser>
) -> (StatusCode, Json<User>) {
    (
        StatusCode::CREATED,
        Json(User {
            id: 1,
            name: data.name,
            email: data.email,
        })
    )
}

/// 更新用户
#[put("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "更新成功", body = User)]
#[u_response(status = 404, description = "用户不存在")]
async fn update_user(
    #[path] id: u32,
    Json(data): Json<CreateUser>,
) -> Json<User> {
    Json(User {
        id,
        name: data.name,
        email: data.email,
    })
}

/// 删除用户
#[delete("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 204, description = "删除成功")]
#[u_response(status = 404, description = "用户不存在")]
async fn delete_user(#[path] id: u32) -> StatusCode {
    StatusCode::NO_CONTENT
}

// ========== OpenAPI ==========

#[derive(OpenApi)]
#[openapi(
    info(
        title = "User API",
        version = "1.0.0",
        description = "用户管理 API 文档",
        contact(
            name = "API Support",
            email = "support@example.com"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "开发环境"),
        (url = "https://api.example.com", description = "生产环境")
    ),
    tags(
        (name = "用户管理", description = "用户相关的 CRUD 操作")
    )
)]
struct ApiDoc;

#[route("/openapi.json", method = "get")]
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[route("/scalar", method = "get")]
async fn scalar_ui() -> impl IntoResponse {
    Scalar::new("/openapi.json").into_response()
}

#[miko]
async fn main() {
    println!("🚀 Server running on http://localhost:8080");
    println!("📚 Scalar UI:    http://localhost:8080/scalar");
    println!("📄 OpenAPI JSON: http://localhost:8080/openapi.json");
}
```

## utoipa 文档

Miko 的 OpenAPI 集成基于 [utoipa](https://docs.rs/utoipa/) 库。更多高级用法请参考:

- **Schema 定义**: [utoipa ToSchema](https://docs.rs/utoipa/latest/utoipa/derive.ToSchema.html)
- **OpenAPI 配置**: [utoipa OpenApi](https://docs.rs/utoipa/latest/utoipa/derive.OpenApi.html)
- **完整文档**: [utoipa 官方文档](https://docs.rs/utoipa/)

## 下一步

- ✅ 学习 [数据验证](数据验证.md) 提升API质量
- 🔍 了解 [请求提取器](请求提取器.md) 的用法
- 📖 查看 [路由系统](路由系统.md) 定义路由
