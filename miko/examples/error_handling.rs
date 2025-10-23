/// 错误处理示例
///
/// 展示如何使用 Miko 的统一错误处理系统
use miko::app::Application;
use miko::extractor::{Json, Path, Query};
use miko::http::response::into_response::IntoResponse;
use miko::router::Router;
use miko::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, ToSchema)]
struct CreateUserDto {
    name: String,
    email: String,
    age: u8,
}

#[derive(Debug, Serialize, ToSchema)]
struct User {
    id: u32,
    name: String,
    email: String,
    age: u8,
}

#[derive(Debug, Deserialize, ToSchema)]
struct PaginationQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

/// 示例 1: 使用 AppResult 返回类型
#[route("/users", method = "post")]
async fn create_user(Json(data): Json<CreateUserDto>) -> AppResult<Json<User>> {
    // 验证
    if data.name.is_empty() {
        return Err(AppError::ValidationError(vec![
            ValidationErrorDetail::required("name"),
        ]));
    }

    if !data.email.contains('@') {
        return Err(AppError::ValidationError(vec![
            ValidationErrorDetail::invalid_format("email", "valid email address"),
        ]));
    }

    if data.age < 18 {
        return Err(AppError::ValidationError(vec![
            ValidationErrorDetail::min_value("age", 18),
        ]));
    }

    // 模拟业务逻辑
    let user = User {
        id: 1,
        name: data.name,
        email: data.email,
        age: data.age,
    };

    Ok(Json(user))
}

/// 示例 2: 返回不同类型的错误
#[route("/users/{:id}", method = "get")]
async fn get_user(Path(id): Path<u32>) -> AppResult<Json<User>> {
    // 模拟数据库查询
    if id == 0 {
        return Err(AppError::BadRequest("Invalid user ID".to_string()));
    }

    if id > 1000 {
        return Err(AppError::NotFound(format!("User {} not found", id)));
    }

    // 模拟权限检查
    if id == 999 {
        return Err(AppError::Forbidden(
            "You don't have permission to view this user".to_string(),
        ));
    }

    // 模拟服务器错误
    if id == 500 {
        return Err(AppError::InternalServerError(
            "Database connection failed".to_string(),
        ));
    }

    Ok(Json(User {
        id,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        age: 25,
    }))
}

/// 示例 3: 多个验证错误
#[route("/users/{:id}", method = "put")]
async fn update_user(
    Path(id): Path<u32>,
    Json(data): Json<CreateUserDto>,
) -> AppResult<Json<User>> {
    let mut errors = Vec::new();

    // 收集所有验证错误
    if data.name.is_empty() {
        errors.push(ValidationErrorDetail::required("name"));
    } else if data.name.len() > 50 {
        errors.push(ValidationErrorDetail::invalid_length("name", 1, 50));
    }

    if !data.email.contains('@') {
        errors.push(ValidationErrorDetail::invalid_format(
            "email",
            "valid email",
        ));
    }

    if data.age < 18 {
        errors.push(ValidationErrorDetail::min_value("age", 18));
    } else if data.age > 120 {
        errors.push(ValidationErrorDetail::max_value("age", 120));
    }

    // 如果有错误，一次性返回所有错误
    if !errors.is_empty() {
        return Err(AppError::ValidationError(errors));
    }

    Ok(Json(User {
        id,
        name: data.name,
        email: data.email,
        age: data.age,
    }))
}

/// 示例 4: 使用自定义错误
#[route("/users/{:id}", method = "delete")]
async fn delete_user(Path(id): Path<u32>) -> AppResult<impl IntoResponse> {
    if id == 1 {
        return Err(AppError::custom(
            hyper::StatusCode::CONFLICT,
            "CANNOT_DELETE_ADMIN",
            "Cannot delete admin user",
        ));
    }

    Ok("User deleted successfully")
}

/// 示例 5: 外部服务错误
#[route("/users/{:id}/avatar", method = "get")]
async fn get_user_avatar(Path(id): Path<u32>) -> AppResult<impl IntoResponse> {
    // 模拟调用外部存储服务
    if id == 404 {
        return Err(AppError::ExternalServiceError {
            service: "S3".to_string(),
            message: "File not found in storage".to_string(),
        });
    }

    Ok("Avatar URL")
}

/// 示例 6: 自动转换 JSON 错误
#[route("/parse-test", method = "post")]
async fn parse_json_test(Json(data): Json<CreateUserDto>) -> AppResult<impl IntoResponse> {
    // 如果 JSON 格式错误，会自动转换为 AppError::JsonParseError
    Ok(format!("Received: {}", data.name))
}

/// 示例 7: 列表查询带验证
#[route("/users", method = "get")]
async fn list_users(Query(query): Query<PaginationQuery>) -> AppResult<Json<Vec<User>>> {
    let _page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    if page_size > 100 {
        return Err(AppError::BadRequest(
            "Page size cannot exceed 100".to_string(),
        ));
    }

    if page_size == 0 {
        return Err(AppError::BadRequest(
            "Page size must be at least 1".to_string(),
        ));
    }

    // 模拟返回用户列表
    Ok(Json(vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            age: 25,
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            age: 30,
        },
    ]))
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let mut router = Router::new();

    // 注册路由
    router
        .post("/users", create_user)
        .get("/users", list_users)
        .get("/users/{:id}", get_user)
        .put("/users/{:id}", update_user)
        .delete("/users/{:id}", delete_user)
        .get("/users/{:id}/avatar", get_user_avatar)
        .post("/parse-test", parse_json_test);
    println!("{:?}", router.path_map);

    println!("🚀 Server started at http://127.0.0.1:3000");
    println!("\n📝 试试这些请求:");
    println!("  POST   http://127.0.0.1:3000/users");
    println!("  GET    http://127.0.0.1:3000/users");
    println!("  GET    http://127.0.0.1:3000/users/1");
    println!("  GET    http://127.0.0.1:3000/users/999  (403 Forbidden)");
    println!("  GET    http://127.0.0.1:3000/users/9999 (404 Not Found)");
    println!("  PUT    http://127.0.0.1:3000/users/1");
    println!("  DELETE http://127.0.0.1:3000/users/1    (409 Conflict)");
    println!("\n💡 使用以下 JSON 测试 POST /users:");
    println!(r#"  {{"name": "", "email": "invalid", "age": 15}}"#);
    println!("  应该返回验证错误!\n");

    Application::new_(router).run().await.unwrap();
}
