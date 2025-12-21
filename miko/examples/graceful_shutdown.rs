use miko::app::Application;
use miko::router::Router;
use miko::{macros::*, *};
use std::time::Duration;

// 模拟一个耗时的请求
#[get("/slow")]
async fn slow_handler() -> &'static str {
    tracing::info!("--> 收到慢请求，开始处理 (耗时 5 秒)...");
    tokio::time::sleep(Duration::from_secs(5)).await;
    tracing::info!("--> 慢请求处理完成！");
    "I'm slow but I finished!"
}

#[get("/")]
async fn index() -> &'static str {
    "Hello, Graceful Shutdown!"
}

#[tokio::main]
async fn main() {
    // 初始化日志以便观察过程
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut router = Router::new();
    router.get("/", index).get("/slow", slow_handler);

    println!("🚀 服务已启动 http://localhost:8080");
    println!("🧪 测试方法:");
    println!("   1. 在浏览器访问 http://localhost:8080/slow");
    println!("   2. 立即在终端按 Ctrl+C 停止服务");
    println!("   3. 观察日志，服务应等待请求处理完成后才退出");

    Application::new_(router.take()).run().await.unwrap();
}
