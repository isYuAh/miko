pub mod core;

use crate::toolkit::attr::StrAttrMap;
use hyper::Method;
use miko_core::IntoMethods;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};

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
pub fn build_register_expr(ra: &RouteAttr, fn_name: &Ident) -> TokenStream {
    let path = ra.path.clone();
    let methods = if let Some(method) = ra.method.clone() {
        method
    } else {
        vec![Method::GET]
    };
    let mut stmts = Vec::new();
    for method in &methods {
        let method_name = format_ident!("{}", method.as_str().to_uppercase());
        stmts.push(quote! {router.route(::miko::hyper::Method::#method_name, #path, #fn_name);});
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
