use crate::route::RouteAttr;
use crate::route::core::route_handler;
use crate::toolkit::attr::StrAttrMap;
use crate::toolkit::impl_operation::{get_constructor, inject_deps};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemFn, ItemImpl, TypePath, parse_macro_input};

mod extractor;
mod route;
mod toolkit;

/// 标准路由属性宏（用于自定义路由）
///
/// 用法：在处理请求的函数上使用 `#[route(...)]` 或派生宏如 `#[get(...)]`。
/// 该宏根据属性（path/method）和参数注解生成路由处理器。
///
/// 参数标注：
/// - `#[path]`：从路径中提取（如 `/users/{id}`）；
/// - `#[query]`：从查询字符串构建结构并注入；
/// - `#[body]`：从请求体反序列化（默认 JSON；标记 `str` 可保留为 String）；
/// - `#[dep]`：注入全局依赖（参数类型通常为 `Arc<T>`，需先注册该组件）；
/// - `#[config("key")]`/`#[config(path = "key")]`：从应用配置读取并解析为参数类型。
///
/// 注意：
/// - 仅当同时启用 `auto` feature 且应用通过 `#[miko]` 启动时，框架才会自动收集并注册由这些宏生成的路由；
/// - 若未启用 `auto`，`route`/派生宏及 `#[dep]` 不会触发框架级的自动注册或依赖注入——此时需要在你的初始化代码中手动注册路由与依赖；
///
/// 建议：处理器应声明为 `async fn`；若未显式返回类型，宏会自动设置为实现 `IntoResponse` 的类型。
///
/// 示例：
/// ```rust
/// #[get("/hello/{id}")]
/// async fn hello(#[path] id: i32) -> impl miko::http::response::into_response::IntoResponse {
///     // 处理请求
/// }
/// ```
#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as RouteAttr);
    let fn_item = parse_macro_input!(item as ItemFn);
    route_handler(args, fn_item)
}

/// # Miko宏
/// 自动配置
/// - 展开出#\[tokio::main]
/// - 注册依赖[仅限auto]
/// - 加载配置到_config
/// - 新建router: Router
/// - > 用户代码
/// - 收集定义#\[get]等宏定义的路由并注册
/// - 运行app
#[proc_macro_attribute]
pub fn miko(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let str_attr_map = parse_macro_input!(attr as StrAttrMap);
    let user_statements = &input_fn.block.stmts;
    let set_panic_hook = if str_attr_map.map.contains_key("sse") {
        Some(quote! {
            ::miko::http::response::sse::set_sse_panic_hook();
        })
    } else {
        None
    };
    let dep_init = if cfg!(feature = "auto") {
        quote! {
            ::miko::dependency_container::CONTAINER.get_or_init(|| async {
                ::miko::dependency_container::LazyDependencyContainer::new_()
            }).await;
        }
    } else {
        quote! {}
    };
    quote! {
        #[::miko::tokio::main]
        async fn main() {
            #set_panic_hook
            let mut _config = ::miko::app::config::ApplicationConfig::load_().unwrap_or_default();
            let mut router = ::miko::router::Router::new();
            #dep_init

            #( #user_statements )*

            router.merge(::miko::auto::collect_global_router());
            let app = ::miko::app::Application::new(_config, router.take());
            ::miko::tokio::spawn(async {
                ::miko::dependency_container::CONTAINER.get().unwrap().read().await.prewarm_all().await;
            });
            app.run().await.unwrap();
        }
    }
    .into()
}
macro_rules! derive_route_macro {
    ($macro_name: ident, $method_ident:ident) => {
        #[doc = concat!("简写：等价于 `#[route(..., method = \"", stringify!($method_ident), "\" )]`。\n\n",
                         "仅当启用 `auto` feature 且应用通过 `#[miko]` 启动时，框架才会自动注册由该宏生成的路由；\n",
                         "否则该宏仅生成处理函数，路由需在初始化代码中手动注册。")]
        #[proc_macro_attribute]
        pub fn $macro_name(attr: TokenStream, item: TokenStream) -> TokenStream {
            let mut args = syn::parse_macro_input!(attr as RouteAttr);
            let fn_item = syn::parse_macro_input!(item as ItemFn);
            let method_to_add = ::hyper::Method::$method_ident;
            match &mut args.method {
                Some(existing_methods) => {
                    existing_methods.push(method_to_add);
                }
                None => {
                    args.method = Some(vec![method_to_add]);
                }
            }
            route_handler(args, fn_item)
        }
    };
}

derive_route_macro!(get, GET);
derive_route_macro!(post, POST);
derive_route_macro!(put, PUT);
derive_route_macro!(delete, DELETE);
derive_route_macro!(patch, PATCH);
derive_route_macro!(head, HEAD);
derive_route_macro!(options, OPTIONS);
derive_route_macro!(trace, TRACE);
derive_route_macro!(connect, CONNECT);

#[cfg(feature = "auto")]
/// 组件宏：将 `impl` 中的构造函数注册为可由框架管理的可注入组件。
///
/// 使用：
/// - 在 `impl` 上添加 `#[component]`（可带 `prewarm`）以将该类型注册为预热组件；
/// - 构造函数应为 `async fn new(...) -> Self`；构造函数的参数可以声明其它组件的依赖（以 `Arc<T>` 形式）；
/// - 注册后的组件可在处理器参数上使用 `#[dep]` 标注注入（当启用 `auto` 时）。
///
/// `prewarm` 生效条件：仅在应用通过 `#[miko]` 启动（并启用 `auto`）时才会在启动阶段触发预热。
///
/// 示例：
/// ```rust
/// #[component(prewarm)]
/// impl MyService {
///     async fn new(dep: std::sync::Arc<Other>) -> Self { /* ... */ }
/// }
///
/// // 在处理器中注入：
/// async fn handler(#[dep] svc: std::sync::Arc<MyService>) { /* ... */ }
/// ```
#[proc_macro_attribute]
pub fn component(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as StrAttrMap);
    let input_struct = parse_macro_input!(input as ItemImpl);
    let prewarm = args.get("prewarm").is_some();
    let mut depend_get_stmts = Vec::new();
    let mut arg_idents = Vec::new();
    let type_ident = match *input_struct.self_ty.clone() {
        syn::Type::Path(TypePath { path, .. }) => path
            .segments
            .last()
            .map(|seg| seg.ident.clone())
            .unwrap_or_else(|| format_ident!("UnknowType")),
        _ => format_ident!("UnknowType"),
    };
    if let Some(method) = get_constructor(&input_struct.items) {
        if method.sig.asyncness.is_none() {
            panic!("service method new must be async")
        }
        let args = &method.sig.inputs;
        inject_deps(args, &mut depend_get_stmts, &mut arg_idents);
    }
    quote! {
        #input_struct
        ::miko::inventory::submit! {
            ::miko::dependency_container::DependencyDefFn(|| {
                ::miko::dependency_container::DependencyDef {
                    type_id: std::any::TypeId::of::<#type_ident>(),
                    prewarm: #prewarm,
                    name: "___",
                    init_fn: || {
                        Box::pin(async move {
                            #(#depend_get_stmts)*
                            let val: #type_ident = #type_ident::new(#(#arg_idents),*).await;
                            ::std::sync::Arc::new(val) as ::std::sync::Arc<dyn ::std::any::Any + Send + Sync>
                        })
                    }
                }
            })
        }
    }.into()
}
