//! # Miko Utoipa 集成示例
//!
//! 展示如何使用 miko 的 utoipa 功能自动生成 OpenAPI 文档
//!
//! 运行: cargo run --example utoipa_demo --features full,utoipa
//! 访问: http://localhost:3000/scalar

use miko::extractor::Json;
use miko::http::response::into_response::{Html, IntoResponse};
use miko::*;
use serde::{Deserialize, Serialize};

// 使用 miko 重导出的 utoipa,不需要单独引入
#[derive(Debug, Serialize, Deserialize, miko::ToSchema)]
struct User {
    #[schema(example = 1)]
    id: i32,

    #[schema(example = "张三")]
    name: String,

    #[schema(example = "zhangsan@example.com")]
    email: String,
}

#[derive(Debug, Serialize, miko::ToSchema)]
struct ErrorResponse {
    code: i32,
    message: String,
}

/// 获取用户信息
///
/// 根据用户 ID 从数据库查询并返回用户详细信息。
/// 如果用户不存在，返回 404 错误。
#[get("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功返回用户信息", body = User)]
#[u_response(status = 404, description = "用户不存在", body = ErrorResponse)]
#[u_response(status = 500, description = "服务器内部错误")]
async fn get_user(
    #[path]
    #[desc("用户ID")]
    id: String,
) -> impl IntoResponse {
    // 模拟查询用户
    let user_id: i32 = id.parse().unwrap_or(1);
    let user = User {
        id: user_id,
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
    };

    Json(user)
}

/// 创建新用户
///
/// 接收用户数据并创建新用户记录
#[post("/users")]
#[u_tag("用户管理")]
#[u_response(status = 201, description = "创建成功", body = User)]
#[u_response(status = 400, description = "请求数据无效", body = ErrorResponse)]
async fn create_user(#[body] user: User) -> impl IntoResponse {
    // 模拟创建用户
    Json(user)
}

/// 搜索用户
///
/// 根据查询参数搜索用户列表
#[get("/users")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功返回用户列表")]
async fn list_users(
    #[query]
    #[desc("用户名筛选")]
    name: Option<String>,
    #[query]
    #[desc("页码，从1开始")]
    page: Option<i32>,
) -> impl IntoResponse {
    // 模拟搜索
    let users = vec![
        User {
            id: 1,
            name: name.clone().unwrap_or("张三".to_string()),
            email: "zhangsan@example.com".to_string(),
        },
        User {
            id: 2,
            name: "李四".to_string(),
            email: "lisi@example.com".to_string(),
        },
    ];

    Json(users)
}

/// 更新用户信息
#[put("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "更新成功", body = User)]
#[u_response(status = 404, description = "用户不存在")]
async fn update_user(
    #[path]
    #[desc("用户ID")]
    id: String,
    #[body] user: User,
) -> impl IntoResponse {
    Json(user)
}

/// 删除用户
#[delete("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 204, description = "删除成功")]
#[u_response(status = 404, description = "用户不存在")]
async fn delete_user(
    #[path]
    #[desc("用户ID")]
    id: String,
) -> impl IntoResponse {
    format!("User {} deleted", id)
}

#[derive(miko::OpenApi)]
#[openapi(
    info(
        title = "Miko API 示例",
        version = "1.0.0",
        description = "使用 Miko 框架和 Utoipa 生成的 OpenAPI 文档示例",
    ),
    paths(
        get_user,
        create_user,
        list_users,
        update_user,
        delete_user,
    ),
    components(
        schemas(User, ErrorResponse)
    ),
    tags(
        (name = "用户管理", description = "用户相关的 API 端点")
    )
)]
struct ApiDoc;

/// 提供 Scalar UI
#[route("/scalar", method = "get")]
async fn scalar_ui() -> impl IntoResponse {
    use miko::OpenApi;
    let openapi = ApiDoc::openapi();
    let html_content = utoipa_scalar::Scalar::new(openapi).to_html();
    Html(html_content)
}

/// 提供 OpenAPI JSON
#[route("/openapi.json", method = "get")]
async fn openapi_json() -> impl IntoResponse {
    use miko::OpenApi;
    let openapi = ApiDoc::openapi();
    Json(openapi)
}

#[miko]
async fn main() {
    println!("🚀 服务器启动中...");
    println!("📚 Scalar UI:    http://localhost:9999/scalar");
    println!("📄 OpenAPI JSON: http://localhost:9999/openapi.json");
    println!("💡 API 端点:");
    println!("   GET    /users");
    println!("   POST   /users");
    println!("   GET    /users/:id");
    println!("   PUT    /users/:id");
    println!("   DELETE /users/:id");
    println!();
}
