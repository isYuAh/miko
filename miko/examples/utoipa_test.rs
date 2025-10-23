//! # Miko Utoipa 集成测试示例
//! 
//! 简单测试用于验证 utoipa 功能
//! 
//! 运行: cargo check --example utoipa_test --features full,utoipa

use miko_macros::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct User {
    #[schema(example = 1)]
    id: i32,
    
    #[schema(example = "张三")]
    name: String,
}

/// 获取用户信息
/// 
/// 根据用户 ID 返回用户详细信息
#[get("/users/{id}")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功", body = User)]
#[u_response(status = 404, description = "用户不存在")]
async fn get_user(
    #[path] #[desc("用户ID")] id: i32
) {
    println!("Get user: {}", id);
}

/// 搜索用户
#[get("/users")]
#[u_tag("用户管理")]
#[u_response(status = 200, description = "成功")]
async fn list_users(
    #[query] #[desc("用户名筛选")] name: Option<String>,
    #[query] #[desc("页码")] page: Option<i32>
) {
    println!("List users: name={:?}, page={:?}", name, page);
}

/// 创建用户
#[post("/users")]
#[u_tag("用户管理")]
#[u_response(status = 201, description = "创建成功", body = User)]
async fn create_user(
    #[body] user: User
) {
    println!("Create user: {:?}", user);
}

#[derive(utoipa::OpenApi)]
#[openapi(
    info(
        title = "Miko API 测试",
        version = "1.0.0"
    ),
    paths(
        get_user,
        list_users,
        create_user,
    ),
    components(
        schemas(User)
    ),
    tags(
        (name = "用户管理", description = "用户API")
    )
)]
struct ApiDoc;

fn main() {
    use utoipa::OpenApi;
    
    let openapi = ApiDoc::openapi();
    println!("✅ OpenAPI 生成成功!");
    println!("{}", openapi.to_pretty_json().unwrap());
}
