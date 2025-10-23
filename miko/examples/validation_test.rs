/// ValidatedJson 验证测试
///
/// 使用 miko 宏和自动路由注册测试 garde 验证
///
/// 运行: cargo run --example validation_test --features full
use garde::Validate;
use miko::extractor::{Json, ValidatedJson};
use miko::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 用户注册请求 - 带验证
#[derive(Debug, Deserialize, Validate, ToSchema)]
struct CreateUserRequest {
    /// 用户名: 3-50 个字符
    #[garde(length(min = 3, max = 50))]
    username: String,

    /// 邮箱: 必须包含 @ 符号
    #[garde(contains("@"))]
    email: String,

    /// 密码: 至少 8 个字符
    #[garde(length(min = 8))]
    password: String,

    /// 年龄: 13-120 岁
    #[garde(range(min = 13, max = 120))]
    age: u8,
}

/// 用户响应
#[derive(Debug, Serialize)]
struct UserResponse {
    id: u64,
    username: String,
    email: String,
    age: u8,
}

/// ✅ 使用 ValidatedJson - 自动验证
#[post("/users")]
async fn create_user(ValidatedJson(req): ValidatedJson<CreateUserRequest>) -> Json<UserResponse> {
    // 到这里，数据已经通过验证！
    println!("✅ Creating user: {:?}", req);

    Json(UserResponse {
        id: 1,
        username: req.username,
        email: req.email,
        age: req.age,
    })
}

/// 对比: 不使用验证的版本
#[post("/users/unvalidated")]
async fn create_user_unvalidated(Json(req): Json<CreateUserRequest>) -> Json<UserResponse> {
    // ⚠️ 这里没有验证，可能接收到无效数据
    println!("⚠️  Creating user (unvalidated): {:?}", req);

    Json(UserResponse {
        id: 2,
        username: req.username,
        email: req.email,
        age: req.age,
    })
}

/// 嵌套验证示例
#[derive(Debug, Deserialize, Validate)]
struct Address {
    #[garde(length(min = 1))]
    street: String,

    #[garde(length(min = 1))]
    city: String,

    #[garde(length(min = 2, max = 2))]
    country_code: String,
}

#[derive(Debug, Deserialize, Validate)]
struct CreateCompanyRequest {
    #[garde(length(min = 3, max = 100))]
    name: String,

    #[garde(contains("@"))]
    contact_email: String,

    #[garde(dive)] // 嵌套验证
    address: Address,
}

#[post("/companies")]
async fn create_company(ValidatedJson(req): ValidatedJson<CreateCompanyRequest>) -> String {
    println!("✅ Creating company: {:?}", req);
    format!("Company '{}' created successfully", req.name)
}

/// 可选字段验证
#[derive(Debug, Deserialize, Validate)]
struct UpdateProfileRequest {
    #[garde(length(min = 1, max = 100))]
    display_name: Option<String>,

    #[garde(length(min = 1))]
    website: Option<String>,

    #[garde(length(max = 500))]
    bio: Option<String>,
}

#[put("/profile")]
async fn update_profile(ValidatedJson(req): ValidatedJson<UpdateProfileRequest>) -> String {
    println!("✅ Updating profile: {:?}", req);
    "Profile updated successfully".to_string()
}

#[miko]
async fn main() {
    tracing_subscriber::fmt::init();

    println!("🚀 Validation Test Server");
    println!("============================================================");
    println!();
    println!("测试端点:");
    println!("  POST /users              - 创建用户（自动验证）");
    println!("  POST /users/unvalidated  - 创建用户（无验证，对比用）");
    println!("  POST /companies          - 创建公司（嵌套验证）");
    println!("  PUT  /profile            - 更新资料（可选字段验证）");
    println!();
    println!("============================================================");
    println!("测试命令:");
    println!();
    println!("✅ 有效请求:");
    println!(
        r#"curl -X POST http://127.0.0.1:3000/users -H "Content-Type: application/json" -d '{{"username":"alice","email":"alice@example.com","password":"password123","age":25}}'"#
    );
    println!();
    println!("❌ 用户名太短 (应该 >= 3):");
    println!(
        r#"curl -X POST http://127.0.0.1:3000/users -H "Content-Type: application/json" -d '{{"username":"ab","email":"alice@example.com","password":"password123","age":25}}'"#
    );
    println!();
    println!("❌ 邮箱格式错误:");
    println!(
        r#"curl -X POST http://127.0.0.1:3000/users -H "Content-Type: application/json" -d '{{"username":"alice","email":"not-an-email","password":"password123","age":25}}'"#
    );
    println!();
    println!("❌ 密码太短 (应该 >= 8):");
    println!(
        r#"curl -X POST http://127.0.0.1:3000/users -H "Content-Type: application/json" -d '{{"username":"alice","email":"alice@example.com","password":"123","age":25}}'"#
    );
    println!();
    println!("❌ 年龄超出范围 (应该 13-120):");
    println!(
        r#"curl -X POST http://127.0.0.1:3000/users -H "Content-Type: application/json" -d '{{"username":"alice","email":"alice@example.com","password":"password123","age":150}}'"#
    );
    println!();
    println!("✅ 嵌套验证:");
    println!(
        r#"curl -X POST http://127.0.0.1:3000/companies -H "Content-Type: application/json" -d '{{"name":"ACME Corp","contact_email":"info@acme.com","address":{{"street":"123 Main St","city":"New York","country_code":"US"}}}}'"#
    );
    println!();
    println!("❌ 国家代码长度错误 (应该是 2 个字符):");
    println!(
        r#"curl -X POST http://127.0.0.1:3000/companies -H "Content-Type: application/json" -d '{{"name":"ACME Corp","contact_email":"info@acme.com","address":{{"street":"123 Main St","city":"New York","country_code":"USA"}}}}'"#
    );
    println!();
    println!("✅ 可选字段验证:");
    println!(
        r#"curl -X PUT http://127.0.0.1:3000/profile -H "Content-Type: application/json" -d '{{"display_name":"Alice","website":"https://example.com","bio":"Hello world"}}'"#
    );
    println!();
    println!("============================================================");
}
