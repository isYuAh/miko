pub mod core;
pub mod layer;

use crate::toolkit::attr::StrAttrMap;
use hyper::Method;
use miko_core::IntoMethods;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};

pub use layer::LayerAttr;

#[derive(Debug)]
pub struct RouteAttr {
    pub path: String,
    pub method: Option<Vec<Method>>,
}
impl Parse for RouteAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attr_map = StrAttrMap::from_parse_stream(input);
        let path = attr_map.get_or_default("path").unwrap();
        let methods = attr_map.get("method");
        let methods = match methods {
            Some(methods) => methods.into_methods(),
            None => {
                vec![]
            }
        };
        Ok(RouteAttr {
            path,
            method: if methods.is_empty() {
                None
            } else {
                Some(methods)
            },
        })
    }
}

/// 为路由属性生成注册路由到全局路由器（inventory 提交）的代码片段。
///
/// 会根据 `RouteAttr` 中的 method 列表生成对不同 HTTP 方法的 `router.route(...)` 调用。
/// 如果提供了 layers，会自动包装 handler。
pub fn build_register_expr(ra: &RouteAttr, fn_name: &Ident, layers: &[LayerAttr]) -> TokenStream {
    let path = ra.path.clone();
    let methods = if let Some(method) = ra.method.clone() {
        method
    } else {
        vec![Method::GET]
    };

    let mut stmts = Vec::new();

    if layers.is_empty() {
        // 没有 layer，直接注册
        for method in &methods {
            let method_name = format_ident!("{}", method.as_str().to_uppercase());
            stmts.push(quote! {
                router.route(::miko::hyper::Method::#method_name, #path, #fn_name);
            });
        }
    } else {
        // 有 layers，使用已有的 service 方法
        let layer_exprs: Vec<_> = layers.iter().map(|l| &l.layer_expr).collect();

        for method in &methods {
            let _method_name = format_ident!("{}", method.as_str().to_uppercase());
            let service_method_name = format_ident!("{}_service", method.as_str().to_lowercase());
            stmts.push(quote! {
                {
                    let __handler = #fn_name;
                    let __svc = ::miko::handler::handler_to_svc(
                        ::std::sync::Arc::new(
                            ::miko::handler::TypedHandler::new(__handler, ::std::sync::Arc::new(()))
                        )
                    );
                    #(
                        let __svc = {
                            let layered = ::tower::Layer::layer(&#layer_exprs, __svc);
                            ::tower::ServiceExt::map_response(layered, |resp| {
                                let (parts, body) = resp.into_parts();
                                // TODO: This is a temporary solution to pass compilation by assuming the body stream is infallible.
                                // A real error handling mechanism (e.g., converting the error to a 500 response)
                                // should be implemented in the future to replace this.
                                let body = ::http_body_util::BodyExt::map_err(body, |err| {
                                    unreachable!("Body stream error occurred, but was assumed to be infallible: {}", err)
                                });
                                let boxed_body = ::http_body_util::BodyExt::boxed(body);
                                ::miko::hyper::Response::from_parts(parts, boxed_body)
                            })
                        };
                    )*
                    let __boxed = ::tower::util::BoxCloneService::new(__svc);
                    router.#service_method_name(#path, __boxed);
                }
            });
        }
    }

    quote! {
        ::miko::inventory::submit! {
            ::miko::auto::RouteFlag {
                register: |mut router| {
                    #(#stmts)*
                    router
                }
            }
        }
    }
}
