use miko::extractor::Json;
use miko::http::response::into_response::IntoResponse;
use miko::macros::*;
/// 测试 #[prefix] 宏功能
///
/// 这个例子展示如何使用 #[prefix] 宏为模块内的所有路由添加前缀
/// 运行: cargo run --example prefix_test --features auto
use miko::*;
use serde::Serialize;

#[derive(Serialize)]
struct Response {
    path: String,
    message: String,
}

// 不使用 prefix，直接定义路由
#[get("/root")]
async fn root_handler() -> impl IntoResponse {
    Json(Response {
        path: "/root".to_string(),
        message: "This is root".to_string(),
    })
}

// 使用 prefix 宏，为模块内的所有路由添加 /api 前缀
#[prefix("/api")]
mod api {
    use super::*;

    // 实际注册路由为: GET /api/users
    #[get("/users")]
    async fn list_users() -> impl IntoResponse {
        Json(Response {
            path: "/api/users".to_string(),
            message: "List of users".to_string(),
        })
    }

    // 实际注册路由为: GET /api/users/{id}
    #[get("/users/{id}")]
    async fn get_user(
        #[path]
        #[desc("用户ID")]
        id: i32,
    ) -> impl IntoResponse {
        Json(Response {
            path: format!("/api/users/{}", id),
            message: format!("Get user with id: {}", id),
        })
    }

    // 实际注册路由为: POST /api/users
    #[post("/users")]
    async fn create_user() -> impl IntoResponse {
        Json(Response {
            path: "/api/users".to_string(),
            message: "User created".to_string(),
        })
    }

    // 嵌套的模块，会继承外层 /api 前缀
    #[prefix("/admin")]
    mod admin {
        use super::*;

        // 实际注册路由为: GET /api/admin/users
        #[get("/users")]
        async fn admin_list_users() -> impl IntoResponse {
            Json(Response {
                path: "/api/admin/users".to_string(),
                message: "Admin: List of all users".to_string(),
            })
        }

        // 实际注册路由为: DELETE /api/admin/users/{id}
        #[delete("/users/{id}")]
        async fn admin_delete_user(
            #[path]
            #[desc("用户ID")]
            id: i32,
        ) -> impl IntoResponse {
            Json(Response {
                path: format!("/api/admin/users/{}", id),
                message: format!("Admin: User {} deleted", id),
            })
        }
    }
}

// 另一个带有 prefix 的模块
#[prefix("/v2")]
mod v2 {
    use super::*;

    // 实际注册路由为: GET /v2/items
    #[get("/items")]
    async fn list_items() -> impl IntoResponse {
        Json(Response {
            path: "/v2/items".to_string(),
            message: "V2 API: List of items".to_string(),
        })
    }

    // 实际注册路由为: GET /v2/items/{id}
    #[get("/items/{id}")]
    async fn get_item(
        #[path]
        #[desc("物品ID")]
        id: i32,
    ) -> impl IntoResponse {
        Json(Response {
            path: format!("/v2/items/{}", id),
            message: format!("V2 API: Get item with id: {}", id),
        })
    }
}

#[miko]
async fn main() {
    tracing_subscriber::fmt::init();
    println!("=== Prefix Test Server Started ===");
    println!("Available routes:");
    println!("  GET /root                    (no prefix)");
    println!("  GET /api/users");
    println!("  GET /api/users/{{id}}");
    println!("  POST /api/users");
    println!("  GET /api/admin/users         (nested prefix)");
    println!("  DELETE /api/admin/users/{{id}} (nested prefix)");
    println!("  GET /v2/items");
    println!("  GET /v2/items/{{id}}");
    println!("\nTest commands:");
    println!("  curl http://localhost:3000/root");
    println!("  curl http://localhost:3000/api/users");
    println!("  curl http://localhost:3000/api/users/123");
    println!("  curl http://localhost:3000/api/admin/users");
    println!("  curl -X DELETE http://localhost:3000/api/admin/users/456");
    println!("  curl http://localhost:3000/v2/items");
    println!("  curl http://localhost:3000/v2/items/789");
}
