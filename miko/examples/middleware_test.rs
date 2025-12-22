use miko::app::Application;
use miko::router::Router;
use miko::{macros::*, *};
use std::time::Duration;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer, trace::TraceLayer};

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
        // 注意：TimeoutLayer 必须在 CompressionLayer 之前（外层），否则可能统计不到压缩时间，或者超时错误无法被压缩（虽然错误通常很小）
        // 但在这里，顺序主要影响逻辑。如果超时，TimeoutLayer 直接返回错误，里面的 Handler 甚至可能没跑完。
        .with_layer(TimeoutLayer::new(Duration::from_secs(1)))
        // 3. CompressionLayer: 自动 Gzip 压缩 (Infallible, but changes Body type)
        .with_layer(CompressionLayer::new())
        .get("/slow", slow_handler)
        .get("/fast", fast_handler)
        .get("/large", large_handler);

    println!("🚀 Server running on http://localhost:8080");
    println!("🧪 测试方案:");
    println!("  1. 超时测试: curl -v http://localhost:8080/slow");
    println!("     预期: 500 Internal Server Error (或根据 AppError 实现返回具体错误)");
    println!("  2. 正常测试: curl -v http://localhost:8080/fast");
    println!("     预期: 200 OK");
    println!(
        "  3. 压缩测试: curl -v -H 'Accept-Encoding: gzip' http://localhost:8080/large --output - | gunzip | wc -c"
    );
    println!("     预期: 解压后 10240 字节");

    Application::new_(router).run().await.unwrap();
}
