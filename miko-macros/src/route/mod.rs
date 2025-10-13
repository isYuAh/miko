use hyper::Method;
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