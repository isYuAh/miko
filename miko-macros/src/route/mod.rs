pub mod core;

use hyper::Method;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use miko_core::IntoMethods;
use crate::toolkit::attr::StrAttrMap;

#[derive(Debug)]
pub struct RouteAttr {
  pub path: String,
  pub method: Option<Vec<Method>>,
}
impl Parse for RouteAttr {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let attr_map = StrAttrMap::from_parse_stream(input);
    let path = attr_map.get_or_default("path").unwrap();
    let methods = attr_map.get("method").unwrap().into_methods();
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

pub fn build_register_expr(ra: &RouteAttr, fn_name: &Ident) -> TokenStream {
  let path = ra.path.clone();
  let methods = if let Some(method) = ra.method.clone() { method } else { vec![Method::GET] };
  let mut stmts = Vec::new();
  for ref method in methods {
    let method_name  = format_ident!("{}", method.as_str().to_uppercase());
    stmts.push(quote! {router.route(::hyper::Method::#method_name, #path, #fn_name);});
  };
  quote! {
    inventory::submit! {
      ::miko::auto::RouteFlag {
        register: |mut router| {
          #(#stmts)*
          router
        }
      }
    }
  }
}