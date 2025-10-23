/// 自动 Trace ID 示例
///
/// 这个示例展示框架如何自动为每个请求设置 trace_id
/// 无需手动配置中间件,所有错误响应都会自动包含 trace_id
use miko::app::Application;
use miko::error::AppError;
use miko::router::Router;

/// 成功的处理器
async fn success() -> Result<String, AppError> {
    Ok("Success!".to_string())
}

/// 返回 404 错误
async fn not_found() -> Result<String, AppError> {
    Err(AppError::NotFound("Resource not found".to_string()))
}

/// 返回验证错误
async fn validation_error() -> Result<String, AppError> {
    use miko::error::ValidationErrorDetail;

    Err(AppError::ValidationError(vec![
        ValidationErrorDetail {
            field: "email".to_string(),
            message: "Invalid email format".to_string(),
            code: "INVALID_FORMAT".to_string(),
        },
        ValidationErrorDetail {
            field: "password".to_string(),
            message: "Password must be at least 8 characters".to_string(),
            code: "MIN_LENGTH".to_string(),
        },
    ]))
}

/// 返回自定义错误
async fn custom_error() -> Result<String, AppError> {
    Err(AppError::custom(
        hyper::StatusCode::IM_A_TEAPOT,
        "TEAPOT",
        "I'm a teapot",
    ))
}

/// 返回内部错误
async fn internal_error() -> Result<String, AppError> {
    Err(AppError::InternalServerError(
        "Something went wrong".to_string(),
    ))
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let mut router = Router::new();
    router
        .get("/", success)
        .get("/not-found", not_found)
        .get("/validation-error", validation_error)
        .get("/custom-error", custom_error)
        .get("/internal-error", internal_error);

    println!("🚀 Server running on http://127.0.0.1:3000");
    println!();
    println!("测试端点:");
    println!("  GET  /                  - 成功响应 (无 trace_id 在响应体中)");
    println!("  GET  /not-found         - 404 错误 (包含 trace_id)");
    println!("  GET  /validation-error  - 验证错误 (包含 trace_id)");
    println!("  GET  /custom-error      - 自定义错误 (包含 trace_id)");
    println!("  GET  /internal-error    - 内部错误 (包含 trace_id)");
    println!();
    println!("Trace ID 说明:");
    println!("  1. 框架会自动为每个请求设置 trace_id");
    println!("  2. 优先从请求头 x-trace-id 或 x-request-id 获取");
    println!("  3. 如果请求头中没有,则自动生成");
    println!("  4. 所有错误响应都会自动包含 trace_id 字段");
    println!();
    println!("测试示例:");
    println!("  # 不带 trace_id 请求 (自动生成)");
    println!("  curl http://127.0.0.1:3000/not-found");
    println!();
    println!("  # 带自定义 trace_id 请求");
    println!("  curl -H 'x-trace-id: my-custom-trace-123' http://127.0.0.1:3000/validation-error");
    println!();

    Application::new_(router).run().await.unwrap();
}
