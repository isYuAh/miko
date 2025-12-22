use miko::app::Application;
use miko::router::Router;
use miko::{macros::*, *};
use std::time::Duration;
// use tower_http::timeout::TimeoutLayer; // 推荐用于 HTTP 服务，超时返回 408 Response
use tower::timeout::TimeoutLayer;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
// 仅用于演示错误捕获机制，超时返回 BoxError -> 500 Response

#[get("/slow")]
async fn slow_handler() -> &'static str {
    // 模拟耗时操作，超过超时限制
    tokio::time::sleep(Duration::from_secs(2)).await;
    "Success (Should not see this)"
}

#[get("/fast")]
async fn fast_handler() -> &'static str {
    "Fast response"
}

#[get("/large")]
async fn large_handler() -> String {
    // 生成一个大响应，测试 Gzip 压缩
    "A".repeat(1024 * 10) // 10KB
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let mut router = Router::new();

    // 1. TraceLayer: 记录请求日志 (Infallible)
    router
        .with_layer(TraceLayer::new_for_http())
        // 2. TimeoutLayer: 设置 1 秒超时 (Fallible -> AppError)
        // 选项 A: tower_http::timeout::TimeoutLayer (推荐)
        //   - 行为: 超时直接返回 HTTP 408 Request Timeout 响应。
        //   - 优点: 符合 HTTP 语义，无需框架介入。
        //   - .with_layer(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(1)))
        // 选项 B: tower::timeout::TimeoutLayer (测试用)
        //   - 行为: 超时抛出 tower::timeout::error::Elapsed 错误。
        //   - 演示: 错误会被 Miko 捕获，转换为 AppError，最终返回 500 Internal Server Error (JSON)。
        .with_layer(TimeoutLayer::new(Duration::from_secs(1)))
        // 3. CompressionLayer: 自动 Gzip 压缩 (Infallible, but changes Body type)
        .with_layer(CompressionLayer::new())
        .get("/slow", slow_handler)
        .get("/fast", fast_handler)
        .get("/large", large_handler);

    println!("🚀 Server running on http://localhost:8080");
    println!("🧪 测试方案:");
    println!("  1. 超时测试: curl -v http://localhost:8080/slow");
    println!(
        "     预期: 500 Internal Server Error (使用 tower::timeout) 或 408 Request Timeout (使用 tower_http::timeout)"
    );
    println!("  2. 正常测试: curl -v http://localhost:8080/fast");
    println!("     预期: 200 OK");
    println!(
        "  3. 压缩测试: curl -v -H 'Accept-Encoding: gzip' http://localhost:8080/large --output - | gunzip | wc -c"
    );
    println!("     预期: 解压后 10240 字节");

    Application::new_(router).run().await.unwrap();
}
