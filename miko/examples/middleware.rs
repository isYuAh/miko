use miko::{macros::*, *};

#[middleware]
async fn logg(a: i32, #[config("value.haha.heihei")] b: String) -> Result<Resp, AppError> {
    println!("logg active, {}, {}", a, b);
    _next.run(_req).await
}

#[get("/")]
#[layer(logg(88))]
async fn hello() -> &'static str {
    "Hello, world!"
}

#[miko]
async fn main() {}
