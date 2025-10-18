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
pub fn miko(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let user_statements = &input_fn.block.stmts;
    let dep_init: Option<proc_macro2::TokenStream> = None;
    #[cfg(feature = "auto")]
    let dep_init = quote! {
        ::miko::dep::CONTAINER.get_or_init(|| async {
            LazyDependencyContainer::new_()
        }).await;
    };
    quote! {
      #[::tokio::main]
      async fn main() {
        let mut _config = ::miko::config::config::ApplicationConfig::load_().unwrap_or_default();
        let mut router = ::miko::handler::router::Router::new();
        #dep_init

        #( #user_statements )*

        router.merge(::miko::auto::collect_global_router());
        let app = ::miko::application::Application::new(_config, router.take());
        ::tokio::spawn(async {
            ::miko::dep::CONTAINER.get().unwrap().read().await.prewarm_all().await;
        });
        app.run().await.unwrap();
      }
    }
    .into()
}
macro_rules! derive_route_macro {
    ($macro_name: ident, $method_ident:ident) => {
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
        ::inventory::submit! {
            ::miko::dep::DependencyDefFn(|| {
                ::miko::dep::DependencyDef {
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
