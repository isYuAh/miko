use miko::macros::*;
use miko::*;
// 自定义一个简单的 Layer，用于添加响应头
#[derive(Clone)]
struct AddHeaderLayer {
    header_name: &'static str,
    header_value: &'static str,
}

impl AddHeaderLayer {
    fn new(header_name: &'static str, header_value: &'static str) -> Self {
        Self {
            header_name,
            header_value,
        }
    }
}

impl<S> tower::Layer<S> for AddHeaderLayer {
    type Service = AddHeaderService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AddHeaderService {
            inner,
            header_name: self.header_name,
            header_value: self.header_value,
        }
    }
}

#[derive(Clone)]
struct AddHeaderService<S> {
    inner: S,
    header_name: &'static str,
    header_value: &'static str,
}

impl<S> tower::Service<miko_core::Req> for AddHeaderService<S>
where
    S: tower::Service<miko_core::Req, Response = miko_core::Resp> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: miko_core::Req) -> Self::Future {
        let mut inner = self.inner.clone();
        let header_name = self.header_name;
        let header_value = self.header_value;

        Box::pin(async move {
            let mut resp = inner.call(req).await?;
            resp.headers_mut()
                .insert(header_name, header_value.parse().unwrap());
            Ok(resp)
        })
    }
}

#[prefix("/api")]
#[layer(AddHeaderLayer::new("X-Module-Layer", "Applied"))]
mod api {
    use super::*;
    // 注意：避免导入与属性宏同名的模块
    // 如果需要使用 garde::rules::prefix，请使用全路径

    // 示例 1: 单个 layer - 添加自定义响应头
    #[get("/test1")]
    #[layer(AddHeaderLayer::new("X-Custom-Header", "Layer-Applied"))]
    async fn test_single_layer() -> String {
        "Test single layer - check response headers for X-Custom-Header".to_string()
    }

    #[prefix("/inner")]
    #[layer(AddHeaderLayer::new("X-Inner-Layer", "Inner-Applied"))]
    mod inner {
        use super::*;

        #[get("/test_inner")]
        #[layer(AddHeaderLayer::new("X-Route-INNER-Layer", "Inner-Applied"))]
        async fn test_inner_layer() -> String {
            "Test inner module layer - check response headers for X-Inner-Layer".to_string()
        }
    }

    // 示例 2: 多个 layers - 添加多个响应头
    #[get("/test2")]
    #[layer(AddHeaderLayer::new("X-Layer-1", "First"))]
    #[layer(AddHeaderLayer::new("X-Layer-2", "Second"))]
    async fn test_multiple_layers() -> String {
        "Test multiple layers - check response headers for X-Layer-1 and X-Layer-2".to_string()
    }
}

// 定义一个预设的 layer 函数
fn custom_header_layer() -> AddHeaderLayer {
    AddHeaderLayer::new("X-Preset-Layer", "From-Function")
}

// 示例 3: 使用函数调用
#[get("/api/test3")]
#[layer(custom_header_layer())]
async fn test_function_layer() -> String {
    "Test function-based layer - check for X-Preset-Layer header".to_string()
}

// 示例 4: 不使用 layer 的普通路由
#[get("/api/test4")]
async fn test_no_layer() -> String {
    "Test no layer - no custom headers should be present".to_string()
}

// 示例 5: 带路径参数
#[get("/api/users/{id}")]
#[layer(AddHeaderLayer::new("X-User-Route", "true"))]
async fn get_user(#[path] id: i32) -> String {
    format!("User ID: {} - check for X-User-Route header", id)
}

// 示例 6: POST 请求带 layer
#[post("/api/data")]
#[layer(AddHeaderLayer::new("X-Post-Layer", "POST-Applied"))]
async fn post_data(#[body] data: String) -> String {
    format!("Received: {} - check for X-Post-Layer header", data)
}

#[miko]
async fn main() {
    println!("========================================");
    println!("Layer Test Server Starting...");
    println!("========================================");
    println!("\nTest the following endpoints with curl:");
    println!("\n1. Single Layer:");
    println!("   curl -i http://localhost:3000/api/test1");
    println!("   Expected: X-Custom-Header: Layer-Applied");

    println!("\n2. Multiple Layers:");
    println!("   curl -i http://localhost:3000/api/test2");
    println!("   Expected: X-Layer-1: First AND X-Layer-2: Second");

    println!("\n3. Function-based Layer:");
    println!("   curl -i http://localhost:3000/api/test3");
    println!("   Expected: X-Preset-Layer: From-Function");

    println!("\n4. No Layer:");
    println!("   curl -i http://localhost:3000/api/test4");
    println!("   Expected: No X-* custom headers");

    println!("\n5. With Path Parameter:");
    println!("   curl -i http://localhost:3000/api/users/123");
    println!("   Expected: X-User-Route: true");

    println!("\n6. POST with Layer:");
    println!(
        "   curl -i -X POST -H 'Content-Type: text/plain' -d 'test data' http://localhost:3000/api/data"
    );
    println!("   Expected: X-Post-Layer: POST-Applied");

    println!("\n========================================");
}
